use crate::ast::{solfa::PulseAccent, symbols::SymbolRef};

pub type List<T> = Vec<SymbolRef<T>>;
pub type TimedList<T> = List<TimedValue<T>>;

#[derive(Debug, Clone)]
pub struct TimedValue<T> {
    pub start: f32,
    pub end: Option<f32>,
    pub value: T,
}

#[derive(Debug, Default, Clone)]
pub struct TimeSignature {
    pub top: usize,
    pub bottom: usize,
}

impl TimeSignature {
    pub fn get_accent(&self, position: usize) -> PulseAccent {
        if position == 0 {
            return PulseAccent::Strong;
        }

        let group_size = if self.top.is_multiple_of(3) { 3 } else { 2 };

        if position.is_multiple_of(group_size) {
            PulseAccent::Medium
        } else {
            PulseAccent::Weak
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Key {
    pub base: BaseKey,
    pub accidental: KeyAccidental,
}

impl std::fmt::Display for Key {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let suffix = match self.accidental {
            KeyAccidental::Neutral => "",
            KeyAccidental::Sharp => "#",
            KeyAccidental::Flat => "b",
        };

        write!(f, "{:?}{suffix}", self.base)
    }
}

impl TryFrom<String> for Key {
    type Error = ();

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let base_str = value.get(..1).ok_or(())?;
        let accidental_str = value.get(1..).ok_or(())?;
        let base = BaseKey::try_from(base_str)?;
        let accidental = KeyAccidental::try_from(accidental_str).unwrap_or_default();

        Ok(Key { base, accidental })
    }
}

#[derive(Debug, Clone, Copy)]
pub enum BaseKey {
    C,
    D,
    E,
    F,
    G,
    A,
    B,
}

impl TryFrom<&str> for BaseKey {
    type Error = ();

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "C" => Ok(Self::C),
            "D" => Ok(Self::D),
            "E" => Ok(Self::E),
            "F" => Ok(Self::F),
            "G" => Ok(Self::G),
            "A" => Ok(Self::A),
            "B" => Ok(Self::B),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub enum KeyAccidental {
    #[default]
    Neutral,
    Sharp,
    Flat,
}

impl TryFrom<&str> for KeyAccidental {
    type Error = ();

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "b" => Ok(Self::Flat),
            "#" => Ok(Self::Sharp),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Hash)]
pub enum Voice {
    S,
    A,
    T,
    B,
}

impl TryFrom<String> for Voice {
    type Error = ();

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.as_str() {
            "S" => Ok(Self::S),
            "A" => Ok(Self::A),
            "T" => Ok(Self::T),
            "B" => Ok(Self::B),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Dynamic {
    P,
    MP,
    PP,
    PPP,
    F,
    MF,
    FF,
    FFF,
    Cre,
    Dec,
}

impl Dynamic {
    pub fn expected_params(self) -> usize {
        match self {
            Dynamic::Cre | Dynamic::Dec => 2,
            _ => 1,
        }
    }
}

impl TryFrom<String> for Dynamic {
    type Error = ();

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.as_str() {
            "p" => Ok(Self::P),
            "mp" => Ok(Self::MP),
            "pp" => Ok(Self::PP),
            "ppp" => Ok(Self::PPP),
            "f" => Ok(Self::F),
            "mf" => Ok(Self::MF),
            "ff" => Ok(Self::FF),
            "fff" => Ok(Self::FFF),
            "cre" => Ok(Self::Cre),
            "dec" => Ok(Self::Dec),
            _ => Err(()),
        }
    }
}

impl std::fmt::Display for Dynamic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Dynamic::P => write!(f, "p"),
            Dynamic::MP => write!(f, "mp"),
            Dynamic::PP => write!(f, "pp"),
            Dynamic::PPP => write!(f, "ppp"),
            Dynamic::F => write!(f, "f"),
            Dynamic::MF => write!(f, "mf"),
            Dynamic::FF => write!(f, "ff"),
            Dynamic::FFF => write!(f, "fff"),
            Dynamic::Cre => write!(f, "cre"),
            Dynamic::Dec => write!(f, "dec"),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Mark {
    DS,
    DC,
    Segno,
    Coda,
}

impl TryFrom<String> for Mark {
    type Error = ();

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.as_str() {
            "DS" => Ok(Self::DS),
            "DC" => Ok(Self::DS),
            "S" => Ok(Self::Segno),
            "C" => Ok(Self::Coda),
            _ => Err(()),
        }
    }
}

impl std::fmt::Display for Mark {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Mark::DS => write!(f, "DC"),
            Mark::DC => write!(f, "DS"),
            Mark::Segno => write!(f, "S"),
            Mark::Coda => write!(f, "C"),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::ast::{solfa::PulseAccent, types::TimeSignature};

    #[test]
    fn test_time_signature() {
        let test_cases = [
            // | :
            (
                TimeSignature { top: 2, bottom: 4 },
                vec![PulseAccent::Strong, PulseAccent::Weak],
            ),
            // | : :
            (
                TimeSignature { top: 3, bottom: 4 },
                vec![PulseAccent::Strong, PulseAccent::Weak, PulseAccent::Weak],
            ),
            // | : ! :
            (
                TimeSignature { top: 4, bottom: 4 },
                vec![
                    PulseAccent::Strong,
                    PulseAccent::Weak,
                    PulseAccent::Medium,
                    PulseAccent::Weak,
                ],
            ),
            // | : : ! : :
            (
                TimeSignature { top: 6, bottom: 4 },
                vec![
                    PulseAccent::Strong,
                    PulseAccent::Weak,
                    PulseAccent::Weak,
                    PulseAccent::Medium,
                    PulseAccent::Weak,
                    PulseAccent::Weak,
                ],
            ),
        ];

        for (signature, expected) in test_cases {
            let mut pulses = Vec::new();

            for pos in 0..signature.top {
                pulses.push(signature.get_accent(pos));
            }

            assert_eq!(expected, pulses);
        }
    }
}

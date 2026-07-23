use crate::ast::solfa::PulseAccent;

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

impl TryFrom<&str> for Key {
    type Error = ();

    fn try_from(value: &str) -> Result<Self, Self::Error> {
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

impl TryFrom<&str> for Voice {
    type Error = ();

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "S" => Ok(Self::S),
            "A" => Ok(Self::A),
            "T" => Ok(Self::T),
            "B" => Ok(Self::B),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum DynamicKind {
    P,
    MP,
    PP,
    PPP,
    F,
    MF,
    FF,
    FFF,
    DC,
    DS,
    Seg,
    Cre,
    Dec,
}

impl DynamicKind {
    pub fn expected_params(self) -> usize {
        match self {
            DynamicKind::Cre | DynamicKind::Dec => 2,
            _ => 1,
        }
    }
}

impl TryFrom<&str> for DynamicKind {
    type Error = ();

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "p" => Ok(Self::P),
            "mp" => Ok(Self::MP),
            "pp" => Ok(Self::PP),
            "ppp" => Ok(Self::PPP),
            "f" => Ok(Self::F),
            "mf" => Ok(Self::MF),
            "ff" => Ok(Self::FF),
            "fff" => Ok(Self::FFF),
            "dc" => Ok(Self::DC),
            "ds" => Ok(Self::DS),
            "seg" => Ok(Self::Seg),
            "cre" => Ok(Self::Cre),
            "dec" => Ok(Self::Dec),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Dynamic {
    pub kind: DynamicKind,
    pub start: f32,
    pub end: f32,
}

impl Dynamic {
    pub fn is_mark(&self) -> bool {
        self.start == self.end
    }

    pub fn is_range(&self) -> bool {
        self.start < self.end
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

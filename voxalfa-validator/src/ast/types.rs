use crate::ast::solfa::PulseAccent;

#[derive(Debug, Default, Clone)]
pub struct TimeSignature {
    pub top: usize,
    pub bottom: usize,
}

impl TimeSignature {
    pub fn get_accent(&self, position: usize) -> PulseAccent {
        if position == 0 {
            PulseAccent::Strong
        } else if (self.top % 3 == 0 && position % 3 == 0)
            || (self.top % 2 == 0 && position % 2 == 0)
        {
            PulseAccent::Medium
        } else {
            PulseAccent::Weak
        }
    }
}

#[derive(Debug)]
pub struct Key {
    pub base: BaseKey,
    pub accidental: KeyAccidental,
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

#[derive(Debug)]
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

#[derive(Debug, Default)]
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

#[derive(Debug, Clone, Copy, PartialEq)]
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

#[derive(Debug)]
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
            "seg" | "$" => Ok(Self::Seg),
            "cre" => Ok(Self::Cre),
            "dec" => Ok(Self::Dec),
            _ => Err(()),
        }
    }
}

#[derive(Debug)]
pub struct Dynamic {
    pub kind: DynamicKind,
    pub start: usize,
    pub end: usize,
}

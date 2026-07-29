#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Key {
    pub base: BaseKey,
    pub accidental: KeyAccidental,
}

impl Key {
    pub fn offset(self) -> i8 {
        self.base.offset() + self.accidental.offset()
    }
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

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BaseKey {
    C,
    D,
    E,
    F,
    G,
    A,
    B,
}

impl BaseKey {
    fn offset(self) -> i8 {
        match self {
            BaseKey::C => 0,
            BaseKey::D => 2,
            BaseKey::E => 4,
            BaseKey::F => 5,
            BaseKey::G => 7,
            BaseKey::A => 9,
            BaseKey::B => 11,
        }
    }
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

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub enum KeyAccidental {
    #[default]
    Neutral,
    Sharp,
    Flat,
}

impl KeyAccidental {
    fn offset(self) -> i8 {
        match self {
            KeyAccidental::Neutral => 0,
            KeyAccidental::Sharp => 1,
            KeyAccidental::Flat => -1,
        }
    }
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

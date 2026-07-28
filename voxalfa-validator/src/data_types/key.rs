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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExtendedTempo {
    Progressive(ProgressiveTempo),
    Static(StaticTempo),
}

impl TryFrom<String> for ExtendedTempo {
    type Error = ();

    fn try_from(value: String) -> Result<Self, Self::Error> {
        ProgressiveTempo::try_from(value.as_str())
            .map(Self::Progressive)
            .or(StaticTempo::try_from(value).map(Self::Static))
    }
}

impl std::fmt::Display for ExtendedTempo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Static(value) => write!(f, "{value}"),
            Self::Progressive(value) => write!(f, "{value}"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StaticTempo {
    Grave,
    Largo,
    Adagio,
    Andante,
    Moderato,
    Allegro,
    Vivace,
    Presto,
    Custom(u16),
}

impl StaticTempo {
    pub fn bpm(&self) -> u16 {
        match self {
            StaticTempo::Grave => 40,
            StaticTempo::Largo => 50,
            StaticTempo::Adagio => 70,
            StaticTempo::Andante => 90,
            StaticTempo::Moderato => 110,
            StaticTempo::Allegro => 130,
            StaticTempo::Vivace => 160,
            StaticTempo::Presto => 180,
            StaticTempo::Custom(val) => *val,
        }
    }
}

impl std::fmt::Display for StaticTempo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Custom(value) => write!(f, "{value}"),
            _ => write!(f, "{}", format!("{self:?}").to_lowercase()),
        }
    }
}

impl TryFrom<String> for StaticTempo {
    type Error = ();

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.as_str() {
            "grave" => Ok(Self::Grave),
            "largo" => Ok(Self::Largo),
            "adagio" => Ok(Self::Adagio),
            "andante" => Ok(Self::Andante),
            "moderato" => Ok(Self::Moderato),
            "allegro" => Ok(Self::Allegro),
            "vivace" => Ok(Self::Vivace),
            "presto" => Ok(Self::Presto),
            _ => value
                .parse::<usize>()
                .map_err(|_| ())
                .map(|v| Self::Custom(v as u16)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProgressiveTempo {
    Rit,
    Accel,
}

impl ProgressiveTempo {
    pub fn ratio(self) -> f32 {
        match self {
            ProgressiveTempo::Rit => 0.8,
            ProgressiveTempo::Accel => 1.2,
        }
    }
}

impl TryFrom<&str> for ProgressiveTempo {
    type Error = ();

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "rit" => Ok(Self::Rit),
            "accel" => Ok(Self::Accel),
            _ => Err(()),
        }
    }
}

impl std::fmt::Display for ProgressiveTempo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", format!("{self:?}").to_lowercase())
    }
}

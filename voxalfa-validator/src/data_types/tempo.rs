#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tempo {
    Grave,
    Largo,
    Adagio,
    Andante,
    Moderato,
    Allegro,
    Vivace,
    Presto,
    Custom(usize),
}

impl Tempo {
    pub fn bpm(&self) -> usize {
        match self {
            Tempo::Grave => 40,
            Tempo::Largo => 50,
            Tempo::Adagio => 70,
            Tempo::Andante => 90,
            Tempo::Moderato => 110,
            Tempo::Allegro => 130,
            Tempo::Vivace => 160,
            Tempo::Presto => 180,
            Tempo::Custom(val) => *val,
        }
    }
}

impl std::fmt::Display for Tempo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Tempo::Custom(value) => write!(f, "{value}"),
            named => write!(f, "{}", named.bpm()),
        }
    }
}

impl TryFrom<String> for Tempo {
    type Error = ();

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.to_lowercase().as_str() {
            "grave" => Ok(Tempo::Grave),
            "largo" => Ok(Tempo::Largo),
            "adagio" => Ok(Tempo::Adagio),
            "andante" => Ok(Tempo::Andante),
            "moderato" => Ok(Tempo::Moderato),
            "allegro" => Ok(Tempo::Allegro),
            "vivace" => Ok(Tempo::Vivace),
            "presto" => Ok(Tempo::Presto),
            _ => value.parse::<usize>().map_err(|_| ()).map(Self::Custom),
        }
    }
}

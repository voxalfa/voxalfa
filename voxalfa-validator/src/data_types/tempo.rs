#[derive(Debug, Clone, Copy)]
pub enum Tempo {
    Custom(usize),
}

impl std::fmt::Display for Tempo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Tempo::Custom(value) => write!(f, "{value}"),
        }
    }
}

impl TryFrom<String> for Tempo {
    type Error = ();

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.as_str() {
            _ => value.parse::<usize>().map_err(|_| ()).map(Self::Custom),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Touch {
    Staccato,
    Fermata,
    Marcato,
}

impl std::fmt::Display for Touch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Touch::Staccato => write!(f, "staccato"),
            Touch::Fermata => write!(f, "fermata"),
            Touch::Marcato => write!(f, "marcato"),
        }
    }
}

impl TryFrom<String> for Touch {
    type Error = ();

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.as_str() {
            "staccato" => Ok(Self::Staccato),
            "fermata" => Ok(Self::Fermata),
            "marcato" => Ok(Self::Marcato),
            _ => Err(()),
        }
    }
}

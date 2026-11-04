#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Touch {
    Staccato,
    Fermata,
    Marcato,
}

impl std::fmt::Display for Touch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Touch::Staccato => write!(f, "stc"),
            Touch::Fermata => write!(f, "frm"),
            Touch::Marcato => write!(f, "mrc"),
        }
    }
}

impl TryFrom<String> for Touch {
    type Error = ();

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.as_str() {
            "stc" => Ok(Self::Staccato),
            "frm" => Ok(Self::Fermata),
            "mrc" => Ok(Self::Marcato),
            _ => Err(()),
        }
    }
}

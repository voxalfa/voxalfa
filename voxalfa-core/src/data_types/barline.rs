#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Barline {
    Double,
}

impl TryFrom<String> for Barline {
    type Error = ();

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.as_str() {
            "double" => Ok(Self::Double),
            _ => Err(()),
        }
    }
}

impl std::fmt::Display for Barline {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", format!("{self:?}").to_lowercase())
    }
}

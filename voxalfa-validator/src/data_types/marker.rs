#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Marker {
    DS,
    DC,
    Segno,
    Coda,
}

impl TryFrom<String> for Marker {
    type Error = ();

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.as_str() {
            "DS" => Ok(Self::DS),
            "DC" => Ok(Self::DS),
            "S" => Ok(Self::Segno),
            "C" => Ok(Self::Coda),
            _ => Err(()),
        }
    }
}

impl std::fmt::Display for Marker {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Marker::DS => write!(f, "DC"),
            Marker::DC => write!(f, "DS"),
            Marker::Segno => write!(f, "S"),
            Marker::Coda => write!(f, "C"),
        }
    }
}

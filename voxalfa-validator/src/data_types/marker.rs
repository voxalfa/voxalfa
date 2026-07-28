#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Navigation {
    DS,
    DC,
    DSC,
    DSF,
    DCC,
    DCF,
    Segno,
    Coda,
    Fine,
}

impl TryFrom<String> for Navigation {
    type Error = ();

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.as_str() {
            "DS" => Ok(Self::DS),
            "DC" => Ok(Self::DC),
            "DSC" => Ok(Self::DSC),
            "DSF" => Ok(Self::DSF),
            "DCC" => Ok(Self::DCC),
            "DCF" => Ok(Self::DCF),
            "S" => Ok(Self::Segno),
            "C" => Ok(Self::Coda),
            "F" => Ok(Self::Fine),
            _ => Err(()),
        }
    }
}

impl std::fmt::Display for Navigation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Navigation::DS => write!(f, "DS"),
            Navigation::DC => write!(f, "DC"),
            Navigation::DSC => write!(f, "DSC"),
            Navigation::DSF => write!(f, "DSF"),
            Navigation::DCC => write!(f, "DCC"),
            Navigation::DCF => write!(f, "DCF"),
            Navigation::Segno => write!(f, "S"),
            Navigation::Coda => write!(f, "C"),
            Navigation::Fine => write!(f, "F"),
        }
    }
}

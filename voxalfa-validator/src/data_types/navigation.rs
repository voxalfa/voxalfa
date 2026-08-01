#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Jump {
    DS,
    DC,
    DSC,
    DSF,
    DCC,
    DCF,
}

impl Jump {
    pub fn final_mark(&self) -> Option<Mark> {
        match self {
            Jump::DSF | Jump::DCF => Some(Mark::Fine),
            _ => None,
        }
    }

    pub fn mark(&self) -> Mark {
        match self {
            Jump::DS | Jump::DSC | Jump::DSF => Mark::Segno,
            Jump::DC | Jump::DCC | Jump::DCF => Mark::Coda,
        }
    }
}

impl TryFrom<String> for Jump {
    type Error = ();

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.as_str() {
            "DS" => Ok(Self::DS),
            "DC" => Ok(Self::DC),
            "DSC" => Ok(Self::DSC),
            "DSF" => Ok(Self::DSF),
            "DCC" => Ok(Self::DCC),
            "DCF" => Ok(Self::DCF),
            _ => Err(()),
        }
    }
}

impl std::fmt::Display for Jump {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DS => write!(f, "DS"),
            Self::DC => write!(f, "DC"),
            Self::DSC => write!(f, "DSC"),
            Self::DSF => write!(f, "DSF"),
            Self::DCC => write!(f, "DCC"),
            Self::DCF => write!(f, "DCF"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mark {
    ToCoda,
    Segno,
    Coda,
    Fine,
}

impl TryFrom<String> for Mark {
    type Error = ();

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.as_str() {
            "S" => Ok(Self::Segno),
            "C" => Ok(Self::Coda),
            "F" => Ok(Self::Fine),
            "TC" => Ok(Self::Fine),
            _ => Err(()),
        }
    }
}

impl std::fmt::Display for Mark {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Segno => write!(f, "S"),
            Self::Coda => write!(f, "C"),
            Self::Fine => write!(f, "F"),
            Self::ToCoda => write!(f, "TC"),
        }
    }
}

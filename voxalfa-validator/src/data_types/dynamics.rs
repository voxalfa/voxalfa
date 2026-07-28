#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Dynamic {
    P,
    MP,
    PP,
    PPP,
    F,
    MF,
    FF,
    FFF,
    Cre,
    Dec,
}

impl Dynamic {
    pub fn expected_params(self) -> usize {
        match self {
            Dynamic::Cre | Dynamic::Dec => 2,
            _ => 1,
        }
    }
}

impl TryFrom<String> for Dynamic {
    type Error = ();

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.as_str() {
            "p" => Ok(Self::P),
            "mp" => Ok(Self::MP),
            "pp" => Ok(Self::PP),
            "ppp" => Ok(Self::PPP),
            "f" => Ok(Self::F),
            "mf" => Ok(Self::MF),
            "ff" => Ok(Self::FF),
            "fff" => Ok(Self::FFF),
            "cre" => Ok(Self::Cre),
            "dec" => Ok(Self::Dec),
            _ => Err(()),
        }
    }
}

impl std::fmt::Display for Dynamic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Dynamic::P => write!(f, "p"),
            Dynamic::MP => write!(f, "mp"),
            Dynamic::PP => write!(f, "pp"),
            Dynamic::PPP => write!(f, "ppp"),
            Dynamic::F => write!(f, "f"),
            Dynamic::MF => write!(f, "mf"),
            Dynamic::FF => write!(f, "ff"),
            Dynamic::FFF => write!(f, "fff"),
            Dynamic::Cre => write!(f, "cre"),
            Dynamic::Dec => write!(f, "dec"),
        }
    }
}

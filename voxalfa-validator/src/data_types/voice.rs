#[derive(Debug, Clone, Copy, PartialEq, Hash)]
pub enum Voice {
    S,
    A,
    T,
    B,
}

impl Voice {
    pub fn offset(self) -> i8 {
        match self {
            Voice::S => 12,
            Voice::A => 0,
            Voice::T => 0,
            Voice::B => -12,
        }
    }
}

impl TryFrom<String> for Voice {
    type Error = ();

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.as_str() {
            "S" => Ok(Self::S),
            "A" => Ok(Self::A),
            "T" => Ok(Self::T),
            "B" => Ok(Self::B),
            _ => Err(()),
        }
    }
}

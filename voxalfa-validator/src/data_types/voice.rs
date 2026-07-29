#[derive(Debug, Clone, Copy, PartialEq, Hash)]
pub enum Voice {
    S,
    A,
    T,
    B,
}

impl Voice {
    pub fn octave_offset(self) -> i8 {
        match self {
            Voice::S => 0,
            Voice::A => -1,
            Voice::T => -2,
            Voice::B => -3,
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

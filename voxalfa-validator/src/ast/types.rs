#[derive(Debug)]
pub struct TimeSignature {
    pub top: usize,
    pub bottom: usize,
}

#[derive(Debug)]
pub struct Key {
    pub base: NoteBase,
    pub accidental: NoteAccidental,
}

impl TryFrom<&str> for Key {
    type Error = ();

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let base_str = value.get(..1).ok_or(())?;
        let accidental_str = value.get(1..).ok_or(())?;
        let base = NoteBase::try_from(base_str)?;
        let accidental = NoteAccidental::try_from(accidental_str)?;

        Ok(Key { base, accidental })
    }
}

#[derive(Debug)]
pub enum NoteBase {
    C,
    D,
    E,
    F,
    G,
    A,
    B,
}

impl TryFrom<&str> for NoteBase {
    type Error = ();

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "C" => Ok(Self::C),
            "D" => Ok(Self::D),
            "E" => Ok(Self::E),
            "F" => Ok(Self::F),
            "G" => Ok(Self::G),
            "A" => Ok(Self::A),
            "B" => Ok(Self::B),
            _ => Err(()),
        }
    }
}

#[derive(Debug)]
pub enum NoteAccidental {
    Sharp,
    Flat,
    Neutral,
}

impl TryFrom<&str> for NoteAccidental {
    type Error = ();

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "" => Ok(Self::Neutral),
            "b" => Ok(Self::Flat),
            "#" => Ok(Self::Sharp),
            _ => Err(()),
        }
    }
}

#[derive(Debug)]
pub enum Voice {
    S,
    A,
    T,
    B,
}

impl TryFrom<&str> for Voice {
    type Error = ();

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "S" => Ok(Self::S),
            "A" => Ok(Self::A),
            "T" => Ok(Self::T),
            "B" => Ok(Self::B),
            _ => Err(()),
        }
    }
}

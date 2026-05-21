use crate::ast::{symbols::Range, types::Voice};

#[derive(Debug)]
pub enum BaseNote {
    D,
    R,
    M,
    F,
    S,
    L,
    T,
}

#[derive(Debug)]
pub enum NoteVariation {
    Base,
    Raised,
    Lowered,
}

#[derive(Debug)]
pub struct Note {
    pub base: BaseNote,
    pub variation: NoteVariation,
    pub octave: i8,
    pub range: Range,
}

#[derive(Debug)]
pub struct SolfaLine {
    pub voice: Voice,
    pub measures: Vec<Measure>,
    pub range: Range,
}

#[derive(Debug)]
pub struct Measure {
    pub range: Range,
}

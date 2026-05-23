use crate::{ast::types::Voice, ts_utils::range::Range};

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

impl TryFrom<&str> for BaseNote {
    type Error = ();

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "d" => Ok(Self::D),
            "r" => Ok(Self::R),
            "m" => Ok(Self::M),
            "f" => Ok(Self::F),
            "s" => Ok(Self::S),
            "l" => Ok(Self::L),
            "t" => Ok(Self::T),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Default)]
pub enum NoteVariation {
    #[default]
    Base,
    Raised,
    Lowered,
}

impl TryFrom<&str> for NoteVariation {
    type Error = ();

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "a" => Ok(Self::Raised),
            "i" => Ok(Self::Raised),
            _ => Err(()),
        }
    }
}

#[derive(Debug)]
pub struct Note {
    pub base: BaseNote,
    pub variation: NoteVariation,
    pub octave: i8,
}

#[derive(Debug)]
pub struct SolfaLine {
    pub id: usize,
    pub voice: Voice,
    pub measures: Vec<Measure>,
    pub range: Range,
}

#[derive(Debug)]
pub struct Measure {
    pub range: Range,
    pub tokens: Vec<MeasureToken>,
}

impl Measure {
    pub fn new(range: Range) -> Self {
        Self {
            range,
            tokens: Vec::new(),
        }
    }
}

#[derive(Debug)]
pub struct MeasureToken {
    pub range: Range,
    pub kind: MeasureTokenKind,
}

#[derive(Debug)]
pub enum MeasureTokenKind {
    Note(Note),
    EmptyNote,
    ProlongedNote,
    NormalDivision,
    MediumDivision,
    HalfDivision,
    QuarterDivision,
    UnderlineMarker,
}

#[derive(Debug)]
pub struct MeasureState {
    pub col_acc: Vec<usize>,
    pub col_start: Option<Range>,
    pub col_end: Option<Range>,
    pub col_count: usize,
}

impl Default for MeasureState {
    fn default() -> Self {
        Self {
            col_acc: vec![0],
            col_start: None,
            col_end: None,
            col_count: 0,
        }
    }
}

impl MeasureState {
    pub fn is_valid(&self) -> bool {
        self.col_acc.len() > 1 || self.col_acc[0] == 1
    }
}

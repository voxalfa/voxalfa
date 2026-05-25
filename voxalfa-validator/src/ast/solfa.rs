use crate::{
    ast::{
        symbols::{ScopeId, SymbolRef},
        types::Voice,
    },
    ts_utils::range::Range,
};

#[derive(Debug)]
pub struct SolfaLine {
    pub sid: ScopeId,
    pub voice: Voice,
    pub measures: Vec<Measure>,
}

#[derive(Debug)]
pub struct Measure {
    pub sid: ScopeId,
    pub tokens: Vec<MeasureToken>,
}

impl Measure {
    pub fn new(sid: ScopeId) -> Self {
        Self {
            sid,
            tokens: Vec::new(),
        }
    }
}

pub type MeasureToken = SymbolRef<MeasureTokenKind>;

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

impl MeasureTokenKind {
    pub fn is_beat_separator(&self) -> bool {
        matches!(
            self,
            MeasureTokenKind::NormalDivision | MeasureTokenKind::MediumDivision
        )
    }

    pub fn is_beat_divider(&self) -> bool {
        matches!(
            self,
            MeasureTokenKind::QuarterDivision | MeasureTokenKind::HalfDivision
        )
    }

    pub fn is_beat_boundary(&self) -> bool {
        self.is_beat_separator() || self.is_beat_divider()
    }

    pub fn is_note(&self) -> bool {
        matches!(
            self,
            MeasureTokenKind::Note(_)
                | MeasureTokenKind::ProlongedNote
                | MeasureTokenKind::EmptyNote
        )
    }
}

#[derive(Debug, Default)]
pub struct MeasureState {
    pub col_acc: Vec<usize>,
    pub col_start: Option<Range>,
    pub col_end: Option<Range>,
    pub col_count: usize,
    pub last_token_kind: Option<MeasureTokenKind>,
}

impl MeasureState {
    pub fn new() -> Self {
        Self {
            col_acc: vec![0],
            ..Default::default()
        }
    }
}

impl MeasureState {
    pub fn update_range(&mut self, range: Range) {
        if self.col_start.is_none() {
            self.col_start = Some(range);
        }

        self.col_end = Some(range);
    }

    pub fn divide(&mut self) {
        self.col_acc.push(0);
    }

    pub fn append_note(&mut self) {
        if let Some(last) = self.col_acc.last_mut() {
            *last += 1;
        }
    }

    pub fn next_column(&mut self) {
        self.col_acc = vec![0];
        self.col_start = None;
        self.col_count += 1;
    }

    pub fn finalize(&mut self) {
        self.col_count += 1;
    }

    pub fn is_valid(&self) -> bool {
        self.col_acc.len() > 1 || self.col_acc[0] == 1
    }

    pub fn is_empty(&self) -> bool {
        self.col_acc.len() == 1 && self.col_acc[0] == 0
    }
}

#[derive(Debug, Clone, Copy)]
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

#[derive(Debug, Default, Clone, Copy)]
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

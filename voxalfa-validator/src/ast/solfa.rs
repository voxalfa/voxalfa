use crate::ast::{
    symbols::{ScopeId, SymbolRef},
    types::Voice,
};

#[derive(Debug)]
pub struct SolfaLine {
    pub sid: ScopeId,
    pub voice: Voice,
    pub pulses: Vec<Pulse>,
}

#[derive(Debug)]
pub struct Pulse {
    pub sid: ScopeId,
    pub accent: PulseAccent,
    pub tokens: Vec<PulseToken>,
}

#[derive(Debug, Clone, Copy)]
pub enum PulseAccent {
    Strong, // |
    Medium, // !
    Weak,   // :
}

pub type PulseToken = SymbolRef<PulseTokenKind>;

#[derive(Debug)]
pub enum PulseTokenKind {
    Note(Note),
    EmptyNote,
    ProlongedNote,
    HalfDivision,
    QuarterDivision,
    UnderlineMarker,
}

impl PulseTokenKind {
    pub fn is_beat_divider(&self) -> bool {
        matches!(
            self,
            PulseTokenKind::QuarterDivision | PulseTokenKind::HalfDivision
        )
    }

    pub fn is_note(&self) -> bool {
        matches!(
            self,
            PulseTokenKind::Note(_) | PulseTokenKind::ProlongedNote | PulseTokenKind::EmptyNote
        )
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
            "a" => Ok(Self::Lowered),
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

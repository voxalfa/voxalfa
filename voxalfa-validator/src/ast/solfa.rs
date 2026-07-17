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
    pub accent: SymbolRef<PulseAccent>,
    pub tokens: Vec<PulseToken>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PulseAccent {
    Strong, // |
    Medium, // !
    Weak,   // :
}

impl std::fmt::Display for PulseAccent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PulseAccent::Strong => write!(f, "|"),
            PulseAccent::Medium => write!(f, "!"),
            PulseAccent::Weak => write!(f, ":"),
        }
    }
}

pub type PulseToken = SymbolRef<PulseTokenKind>;

#[derive(Debug)]
pub enum PulseTokenKind {
    Note(Note),
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
            PulseTokenKind::Note(_) | PulseTokenKind::ProlongedNote
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

#[derive(Debug, Clone, Copy)]
pub struct Note {
    pub base: BaseNote,
    pub variation: NoteVariation,
    pub octave: i8,
}

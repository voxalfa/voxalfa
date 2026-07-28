use crate::{
    ast::symbols::{ScopeId, SymbolRef},
    data_types::Voice,
    render::RenderType,
};

#[derive(Debug)]
pub struct SolfaLine {
    pub sid: ScopeId,
    pub voice: SymbolRef<Voice>,
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

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BaseNote {
    D,
    R,
    M,
    F,
    S,
    L,
    T,
}

impl std::fmt::Display for BaseNote {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BaseNote::D => write!(f, "d"),
            BaseNote::R => write!(f, "r"),
            BaseNote::M => write!(f, "m"),
            BaseNote::F => write!(f, "f"),
            BaseNote::S => write!(f, "s"),
            BaseNote::L => write!(f, "l"),
            BaseNote::T => write!(f, "t"),
        }
    }
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

#[derive(Debug, Default, Clone, Copy, PartialEq)]
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

impl std::fmt::Display for Note {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let variation_str = match self.variation {
            NoteVariation::Base => "",
            NoteVariation::Raised => "i",
            NoteVariation::Lowered => "a",
        };

        let suffix = match self.octave {
            n if n < 0 => n.to_string(),
            n if n > 0 => format!("+{n}"),
            _ => "".to_string(),
        };

        write!(f, "{}{variation_str}{suffix}", self.base)
    }
}

impl Note {
    pub fn width(&self, render_type: RenderType) -> usize {
        let mut result = 1;

        if self.variation != NoteVariation::Base {
            result += 1;
        }

        if render_type == RenderType::Text && self.octave != 0 {
            result += 2;
        }

        result
    }
}

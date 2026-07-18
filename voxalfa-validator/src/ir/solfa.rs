use crate::{
    ast::{
        solfa::{Note, PulseAccent},
        symbols::ScopeId,
        types::Voice,
    },
    ir::utils::UnderlineRange,
};

#[derive(Debug)]
pub struct SolfaLineIR {
    pub sid: ScopeId,
    pub voice: Voice,
    pub pulses: Vec<PulseIR>,
    pub underlines: Vec<UnderlineRange>,
}
impl SolfaLineIR {
    pub fn new(sid: ScopeId, voice: Voice) -> Self {
        Self {
            sid,
            voice,
            pulses: Vec::new(),
            underlines: Vec::new(),
        }
    }
}

#[derive(Debug)]
pub struct PulseIR {
    pub sid: ScopeId,
    pub accent: PulseAccent,
    pub columns: Vec<PulseColumn>,
    pub factor: usize, // factor of the duration in pulse columns
}

impl PulseIR {
    pub fn new(sid: ScopeId, accent: PulseAccent) -> Self {
        Self {
            sid,
            accent,
            columns: Vec::new(),
            factor: 1,
        }
    }

    pub fn add_column(&mut self, kind: PulseColumnKind) {
        self.columns.push(PulseColumn { kind, duration: 0 });
    }

    pub fn set_length(&mut self, length: usize) {
        self.factor = length;
    }

    pub fn fit_durations(&mut self, durations: &[usize]) {
        for (i, duration) in durations.iter().enumerate() {
            self.columns[i].duration = *duration;
        }
    }
}

#[derive(Debug)]
pub struct PulseColumn {
    pub duration: usize,
    pub kind: PulseColumnKind,
}

#[derive(Debug)]
pub enum PulseColumnKind {
    Note(Note),
    ProlongedNote(Note),
    EmptyNote,
}

impl std::fmt::Display for PulseColumnKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PulseColumnKind::Note(note) => write!(f, "{note}"),
            PulseColumnKind::ProlongedNote(_) => write!(f, "-"),
            PulseColumnKind::EmptyNote => write!(f, " "),
        }
    }
}

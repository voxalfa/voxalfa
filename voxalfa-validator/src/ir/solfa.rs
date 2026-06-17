use crate::ast::solfa::{Note, PulseAccent};

#[derive(Debug, Default)]
pub struct SolfaLineIR {
    pub pulses: Vec<PulseIR>,
    pub underlines: Vec<UnderlineRange>,
}

#[derive(Debug)]
pub struct PulseIR {
    pub accent: PulseAccent,
    pub columns: Vec<PulseColumn>,
}

impl PulseIR {
    pub fn new(accent: PulseAccent) -> Self {
        Self {
            accent,
            columns: Vec::new(),
        }
    }

    pub fn add_column(&mut self, kind: PulseColumnKind) {
        self.columns.push(PulseColumn { kind, duration: 0. });
    }
}

#[derive(Debug)]
pub struct UnderlineRange {
    pub start: usize,
    pub end: usize,
}

#[derive(Debug)]
pub struct PulseColumn {
    pub kind: PulseColumnKind,
    pub duration: f32,
}

#[derive(Debug)]
pub enum PulseColumnKind {
    Notes(Vec<Note>),
    ProlongedNote(Note),
    EmptyNote,
}

#[derive(Debug, Clone, Copy)]
pub enum PulseDivider {
    Dot,
    Comma,
    DotComma,
}

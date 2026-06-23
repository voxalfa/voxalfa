use crate::{
    ast::{
        solfa::{Note, PulseAccent},
        types::Voice,
    },
    ir::utils::UnderlineRange,
};

#[derive(Debug)]
pub struct SolfaLineIR {
    pub voice: Voice,
    pub pulses: Vec<PulseIR>,
    pub underlines: Vec<UnderlineRange>,
}
impl SolfaLineIR {
    pub fn new(voice: Voice) -> Self {
        Self {
            voice,
            pulses: Vec::new(),
            underlines: Vec::new(),
        }
    }
}

#[derive(Debug)]
pub struct PulseIR {
    pub accent: PulseAccent,
    pub columns: Vec<PulseColumn>,
    pub length: usize,
}

impl PulseIR {
    pub fn new(accent: PulseAccent) -> Self {
        Self {
            accent,
            columns: Vec::new(),
            length: 1,
        }
    }

    pub fn add_column(&mut self, kind: PulseColumnKind) {
        self.columns.push(PulseColumn { kind, duration: 0 });
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

use crate::{
    ast::{
        solfa::{Note, PulseAccent},
        symbols::ScopeId,
    },
    data_types::Voice,
    ir::utils::{UnderlineMarker, UnderlineRange},
};

#[derive(Debug)]
pub struct SolfaLineIr {
    pub sid: ScopeId,
    pub voice: Voice,
    pub pulses: Vec<PulseIr>,
}
impl SolfaLineIr {
    pub fn new(sid: ScopeId, voice: Voice) -> Self {
        Self {
            sid,
            voice,
            pulses: Vec::new(),
        }
    }

    pub fn fit_underlines(&mut self, underlines: &[UnderlineRange]) {
        let columns = self.pulses.iter_mut().flat_map(|p| &mut p.columns);

        for (column_idx, column) in columns.enumerate() {
            column.underline.left = underlines.iter().any(|u| u.start == column_idx);
            column.underline.right = underlines.iter().any(|u| u.end - 1 == column_idx);
        }
    }
}

#[derive(Debug)]
pub struct PulseIr {
    pub expanded: bool,
    pub sid: ScopeId,
    pub accent: PulseAccent,
    pub columns: Vec<PulseColumn>,
    pub factor: u8, // factor of the duration in pulse columns
}

impl PulseIr {
    pub fn new(sid: ScopeId, accent: PulseAccent) -> Self {
        Self {
            sid,
            accent,
            columns: Vec::new(),
            expanded: false,
            factor: 1,
        }
    }

    pub fn add_column(&mut self, note: NoteKind) {
        self.columns.push(PulseColumn {
            note,
            duration: 0,
            underline: UnderlineMarker::default(),
        });
    }

    pub fn set_length(&mut self, length: u8) {
        self.factor = length;
    }

    pub fn fit_durations(&mut self, durations: &[u8]) {
        for (i, duration) in durations.iter().enumerate() {
            if let Some(column) = self.columns.get_mut(i) {
                column.duration = *duration;
            }
        }
    }
}

#[derive(Debug)]
pub struct PulseColumn {
    pub duration: u8,
    pub underline: UnderlineMarker,
    pub note: NoteKind,
}

impl PulseColumn {
    pub fn is_empty(&self) -> bool {
        matches!(self.note, NoteKind::EmptyNote)
    }
}

#[derive(Debug)]
pub enum NoteKind {
    Note(Note),
    ProlongedNote,
    EmptyNote,
}

pub mod builder;
pub mod lyrics;
pub mod solfa;
pub mod utils;

use lyrics::LyricLineIR;
use solfa::SolfaLineIR;

use crate::{
    ast::{
        params::{SectionParams, SubSectionParams},
        symbols::ScopeId,
    },
    data_types::Voice,
    event::Timestamp,
    ir::solfa::PulseIR,
};

#[derive(Debug, Default)]
pub struct BodyIR {
    pub sections: Vec<SectionIR>,
}

#[derive(Debug, Default)]
pub struct SectionIR {
    pub sid: ScopeId,
    pub items: Vec<SubSectionIR>,
    pub params: SectionParams,
    pub merge: bool,
}

impl SectionIR {
    pub fn get_verses(&self, voice: &Voice) -> Option<&[LyricLineIR]> {
        self.items.iter().find_map(|s| {
            s.solfa
                .iter()
                .any(|s| s.voice == *voice)
                .then_some(s.lyrics.as_slice())
        })
    }
}

#[derive(Debug, Default)]
pub struct SubSectionIR {
    pub sid: ScopeId,
    pub params: SubSectionParams,
    pub views: Vec<PulseView>,
    pub solfa: Vec<SolfaLineIR>,
    pub lyrics: Vec<LyricLineIR>,
}

impl SubSectionIR {
    pub fn width(&self) -> usize {
        self.views.iter().map(|v| v.factor).sum()
    }

    pub fn last_timestamp(&self) -> Timestamp {
        let pulse_id = self.views.len() - 1;

        let note_id = self
            .views
            .last()
            .map(|l| l.durations.len() - 1)
            .unwrap_or_default();

        Timestamp::end(pulse_id, note_id)
    }
}

#[derive(Debug)]
pub struct PulseView {
    pub durations: Vec<usize>,
    pub factor: usize,
    pub aligned: bool,
}

impl PulseView {
    pub fn new(pulse: &PulseIR) -> Self {
        Self {
            durations: pulse.columns.iter().map(|c| c.duration).collect(),
            factor: pulse.factor,
            aligned: true,
        }
    }

    pub fn add(&mut self, pulse: &PulseIR) {
        let duration_match = pulse
            .columns
            .iter()
            .map(|p| p.duration)
            .zip(&self.durations)
            .all(|(lhs, &rhs)| lhs == rhs);

        if pulse.factor != self.factor || !duration_match {
            self.aligned = false;
            self.factor = 1;
            self.durations = vec![1];
        }
    }

    pub fn resolve_widths(&self, col_width: usize, col_factor: usize) -> Vec<usize> {
        self.durations
            .iter()
            .map(|d| (col_width * d * col_factor) / self.factor)
            .collect()
    }
}

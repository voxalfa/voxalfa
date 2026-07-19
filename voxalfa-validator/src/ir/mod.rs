pub mod lyrics;
pub mod solfa;
pub mod utils;

use lyrics::LyricLineIR;
use solfa::SolfaLineIR;

use crate::ir::solfa::PulseIR;

#[derive(Debug, Default)]
pub struct DocumentIR {
    pub sections: Vec<SectionIR>,
}

#[derive(Debug, Default)]
pub struct SectionIR {
    pub solfa: Vec<SolfaLineIR>,
    pub lyrics: Vec<LyricLineIR>,
    pub groups: Vec<SectionGroup>,
}

#[derive(Debug, Default)]
pub struct SectionGroup {
    pub views: Vec<PulseView>,
    pub solfa: Vec<usize>,
    pub lyrics: Vec<usize>,
}

impl SectionGroup {
    pub fn width(&self) -> usize {
        self.views.iter().map(|v| v.factor).sum()
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

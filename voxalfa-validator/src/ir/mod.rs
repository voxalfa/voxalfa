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
        self.views.iter().map(|v| v.min_factor).sum()
    }
}

#[derive(Debug)]
pub struct PulseView {
    pub min_factor: usize,
    pub max_factor: usize,
}

impl Default for PulseView {
    fn default() -> Self {
        Self {
            min_factor: 1,
            max_factor: 1,
        }
    }
}

impl PulseView {
    pub fn update(&mut self, pulse: &PulseIR) {
        if self.min_factor > pulse.factor {
            self.min_factor = pulse.factor;
        }

        if self.max_factor < pulse.factor {
            self.max_factor = pulse.factor;
        }
    }
}

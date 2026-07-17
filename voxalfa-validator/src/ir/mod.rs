pub mod lyrics;
pub mod solfa;
pub mod utils;

use lyrics::LyricLineIR;
use solfa::SolfaLineIR;

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

impl SectionIR {
    pub fn get_common_column(&self, group: usize, pulse: usize) -> Option<usize> {
        let group = self.groups.get(group)?;
        let first_id = group.solfa.first()?;
        let first = self.solfa[*first_id].pulses.get(pulse)?;

        group
            .solfa
            .iter()
            .all(|id| {
                self.solfa[*id].pulses.get(pulse).is_some_and(|p| {
                    p.factor == first.factor && p.columns.len() == first.columns.len()
                })
            })
            .then_some(first.columns.len())
    }
}

#[derive(Debug, Default)]
pub struct SectionGroup {
    pub solfa: Vec<usize>,
    pub lyrics: Vec<usize>,
}

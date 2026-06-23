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

#[derive(Debug)]
pub struct SectionGroup {
    pub solfa: Vec<usize>,
    pub lyrics: Vec<usize>,
}

use crate::ir::solfa::SolfaLineIR;

pub mod solfa;

#[derive(Debug, Default)]
pub struct DocumentIR {
    pub sections: Vec<SectionIR>,
}

#[derive(Debug, Default)]
pub struct SectionIR {
    pub lines: Vec<SolfaLineIR>,
}

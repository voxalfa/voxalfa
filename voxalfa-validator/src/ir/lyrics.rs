use crate::{
    ast::{lyrics::LyricOperatorKind, symbols::LyricStringId},
    ir::utils::UnderlineRange,
};

#[derive(Debug, Default)]
pub struct LyricLineIR {
    pub items: Vec<LyricGroup>,
    pub underlines: Vec<UnderlineRange>,
}

#[derive(Debug)]
pub struct LyricGroup {
    pub chunks: Vec<LyricChunkIR>,
}

#[derive(Debug)]
pub struct LyricChunkIR {
    pub id: Option<LyricStringId>,
    pub operator: Option<LyricOperatorKind>,
    pub span: usize,
}

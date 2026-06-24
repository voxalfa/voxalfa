use crate::{
    ast::{
        lyrics::{LyricOperatorKind, LyricSpecialChar},
        symbols::LyricStringId,
    },
    ir::utils::UnderlineRange,
};

#[derive(Debug, Default)]
pub struct LyricLineIR {
    pub group: usize,
    pub columns: Vec<LyricColumnIR>,
    pub operators: Vec<LyricOperatorKind>,
    pub underlines: Vec<UnderlineRange>,
}

impl LyricLineIR {
    pub fn new(group: usize) -> LyricLineIR {
        LyricLineIR {
            group,
            ..Default::default()
        }
    }
}

#[derive(Debug, Default)]
pub struct LyricColumnIR {
    pub chunks: Vec<LyricChunkIR>,
    pub operators: Vec<LyricOperatorKind>,
    pub span: usize,
}

impl LyricColumnIR {
    pub fn new(span: usize) -> Self {
        Self {
            span,
            ..Default::default()
        }
    }

    pub fn add_chunk(&mut self, partials: Vec<LyricStringIR>) {
        self.chunks.push(LyricChunkIR { partials });
    }
}

#[derive(Debug)]
pub struct LyricChunkIR {
    pub partials: Vec<LyricStringIR>,
}

#[derive(Debug)]
pub enum LyricStringIR {
    Reference(LyricStringId),
    Special(LyricSpecialChar),
}

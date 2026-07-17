use crate::{
    ast::{
        lyrics::{LyricOperatorKind, LyricSpecialChar},
        symbols::{LyricStringId, ScopeId},
    },
    ir::utils::UnderlineRange,
};

#[derive(Debug, Default)]
pub struct LyricLineIR {
    pub sid: ScopeId,
    pub columns: Vec<LyricColumnIR>,
    pub operators: Vec<LyricOperatorKind>,
    pub underlines: Vec<UnderlineRange>,
}

impl LyricLineIR {
    pub fn new(sid: ScopeId) -> Self {
        Self {
            sid,
            ..Default::default()
        }
    }
}

#[derive(Debug, Default)]
pub struct LyricColumnIR {
    pub sid: ScopeId,
    pub chunks: Vec<LyricChunkIR>,
    pub operators: Vec<LyricOperatorKind>,
    pub span: usize,
}

impl LyricColumnIR {
    pub fn new(sid: ScopeId, span: usize) -> Self {
        Self {
            sid,
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

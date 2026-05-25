use crate::ast::symbols::{Field, ScopeId, SymbolRef};

#[derive(Debug)]
pub struct LyricLine {
    pub sid: ScopeId,
    pub group: usize,
    pub verse: usize,
    pub tokens: Vec<LyricToken>,
    pub anchor: Field<LyricAnchor>,
}

#[derive(Debug)]
pub enum LyricToken {
    Column(LyricColumn),
    Operator(LyricOperator),
}

pub type LyricChunk = SymbolRef<LyricChunkKind>;
pub type LyricOperator = SymbolRef<LyricOperatorKind>;

#[derive(Debug)]
pub enum LyricOperatorKind {
    Space,
    Concat,
    Newline,
}

#[derive(Debug)]
pub struct LyricColumn {
    pub span: usize,
    pub chunks: Vec<LyricChunk>,
}

#[derive(Debug)]
pub enum LyricChunkKind {
    Space,
    Concat,
    Newline,
    Placeholder,
    UnderlineMarker,
    String(String),
}

#[derive(Debug)]
pub enum LyricAnchor {
    Newline,
    Space,
    Concat,
}

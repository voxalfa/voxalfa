use crate::ast::symbols::{Field, ScopeId, SymbolRef};

#[derive(Debug)]
pub struct LyricLine {
    pub sid: ScopeId,
    pub group: usize,
    pub verse: usize,
    pub columns: Vec<LyricColumn>,
    pub operators: Vec<LyricOperator>,
    pub anchor: Field<LyricAnchor>,
}

pub type LyricToken = SymbolRef<LyricTokenKind>;
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
    pub chunks: Vec<LyricToken>,
}

#[derive(Debug)]
pub enum LyricTokenKind {
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

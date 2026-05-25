use crate::ast::symbols::{Field, SymbolRef};

#[derive(Debug)]
pub struct LyricLine {
    pub sid: usize,
    pub group: usize,
    pub verse: usize,
    pub tokens: Vec<Vec<LyricToken>>,
    pub anchor: Field<LyricAnchor>,
}

#[derive(Debug)]
pub enum LyricAnchor {
    Newline,
    Space,
    Concat,
}

pub type LyricToken = SymbolRef<LyricTokenKind>;

#[derive(Debug)]
pub enum LyricTokenKind {
    Space,
    Concat,
    Newline,
    Placeholder,
    UnderlineMarker,
    String(String),
}

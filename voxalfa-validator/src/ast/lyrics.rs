use crate::ts_utils::range::Range;

#[derive(Debug)]
pub struct LyricLine {
    pub id: usize,
    pub verse: usize,
    pub tokens: Vec<LyricToken>,
    pub anchor: Option<LyricAnchor>,
    pub range: Range,
}

#[derive(Debug)]
pub enum LyricAnchor {
    Newline,
    Space,
    Concat,
}

#[derive(Debug)]
pub struct LyricToken {
    pub kind: LyricTokenKind,
    pub range: Range,
}

// FIXME: better abstraction
#[derive(Debug)]
pub enum LyricTokenKind {
    Space,
    Concat,
    Newline,
    UnderlineMarker,
    Placeholder,
    String(String),
    Chunk(Vec<LyricToken>),
    Group(Vec<LyricToken>),
}

#[derive(Debug)]
pub struct Lyric {
    pub id: usize,
    pub verse: usize,
    pub tokens: Vec<LyricToken>,
    pub anchor: Option<LyricAnchor>,
}

#[derive(Debug)]
pub enum LyricAnchor {
    Newline,
    Space,
    Concat,
}

#[derive(Debug)]
pub enum LyricToken {
    Space,
    Concat,
    Newline,
    UnderlineMarker,
    Placeholder,
    String(String),
    Group(Vec<LyricToken>),
}

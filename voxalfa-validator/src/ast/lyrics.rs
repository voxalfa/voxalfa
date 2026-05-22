#[derive(Debug)]
pub struct Lyric {
    pub id: usize,
    pub verse: usize,
    pub tokens: Vec<LyricToken>,
}

#[derive(Debug)]
pub enum LyricToken {
    Space,
    Concat,
    Placeholder,
    Chunk(Vec<LyricChunk>),
}

#[derive(Debug)]
pub enum LyricChunk {
    String(String),
    Break,
    Split,
    UnderlineStart,
    UnderlineEnd,
}

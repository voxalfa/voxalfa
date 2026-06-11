use crate::ast::symbols::{Field, ScopeId, SymbolRef};

#[derive(Debug)]
pub struct LyricLine {
    pub sid: ScopeId,
    pub group: usize,
    pub verse: usize,
    pub tokens: Vec<LyricToken>,
    pub anchor: Option<LyricAnchor>,
}

#[derive(Debug)]
pub enum LyricToken {
    Column(LyricColumn),
    Operator(LyricOperator),
}

pub type LyricChunk = SymbolRef<LyricChunkKind>;
pub type LyricOperator = SymbolRef<LyricOperatorKind>;
pub type LyricAnchor = SymbolRef<LyricAnchorKind>;

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
    SpecialChar(LyricSpecialChar),
}

#[derive(Debug)]
pub enum LyricAnchorKind {
    Newline,
    Space,
    Concat,
}

#[derive(Debug)]
pub enum LyricSpecialChar {
    Backslash,
    Tilde,
    Backtick,
    LeftChrevron,
    RightChevron,
    Slash,
    LeftParen,
    RightParen,
    Plus,
    Ampersand,
}

impl TryFrom<&str> for LyricSpecialChar {
    type Error = ();

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "&bls" => Ok(Self::Backslash),
            "&tld" => Ok(Self::Tilde),
            "&btk" => Ok(Self::Backtick),
            "&lch" => Ok(Self::LeftChrevron),
            "&rch" => Ok(Self::RightChevron),
            "&sls" => Ok(Self::Slash),
            "&lpr" => Ok(Self::LeftParen),
            "&rpr" => Ok(Self::RightParen),
            "&pls" => Ok(Self::Plus),
            "&amp" => Ok(Self::Ampersand),
            _ => Err(()),
        }
    }
}

impl std::fmt::Display for LyricSpecialChar {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let char = match self {
            LyricSpecialChar::Backslash => '\\',
            LyricSpecialChar::Tilde => '~',
            LyricSpecialChar::Backtick => '`',
            LyricSpecialChar::LeftChrevron => '>',
            LyricSpecialChar::RightChevron => '<',
            LyricSpecialChar::Slash => '/',
            LyricSpecialChar::LeftParen => '(',
            LyricSpecialChar::RightParen => ')',
            LyricSpecialChar::Plus => '+',
            LyricSpecialChar::Ampersand => '&',
        };

        write!(f, "{char}")
    }
}

use crate::{
    ast::symbols::{LyricStringId, ScopeId, SymbolRef},
    ts_utils::range::Range,
};

#[derive(Debug)]
pub struct LyricLine {
    pub sid: ScopeId,
    pub verse: usize,
    pub tokens: Vec<LyricToken>,
    pub anchor: Option<Range>,
}

#[derive(Debug)]
pub enum LyricToken {
    Column(LyricColumn),
    Operator(LyricOperator),
}

impl LyricToken {
    pub fn is_operator(&self) -> bool {
        matches!(self, LyricToken::Operator(_))
    }
}

pub type LyricChunk = SymbolRef<LyricChunkKind>;
pub type LyricOperator = SymbolRef<LyricOperatorKind>;
pub type LyricString = SymbolRef<LyricStringKind>;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LyricOperatorKind {
    Space,
    Concat,
    Newline,
}

impl LyricOperatorKind {
    pub fn char(self) -> Option<char> {
        match self {
            LyricOperatorKind::Space => Some(' '),
            LyricOperatorKind::Newline => Some('\n'),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub struct LyricColumn {
    pub sid: ScopeId,
    pub span: usize,
    pub chunks: Vec<LyricChunk>,
}

#[derive(Debug)]
pub enum LyricChunkKind {
    Space,
    Newline,
    Placeholder,
    String(Vec<LyricString>),
}

#[derive(Debug)]
pub enum LyricStringKind {
    UnderlineMarker,
    Reference(LyricStringId),
    SpecialChar(LyricSpecialChar),
}

#[derive(Debug, Clone, Copy)]
pub enum LyricSpecialChar {
    Backslash,
    Tilde,
    Backtick,
    LeftChrevron,
    RightChevron,
    Slash,
    LeftParen,
    RightParen,
    At,
    Ampersand,
    Semicolumn,
    Dot,
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
            "&atr" => Ok(Self::At),
            "&amp" => Ok(Self::Ampersand),
            "&scl" => Ok(Self::Semicolumn),
            "&dot" => Ok(Self::Dot),
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
            LyricSpecialChar::At => '@',
            LyricSpecialChar::Ampersand => '&',
            LyricSpecialChar::Semicolumn => ';',
            LyricSpecialChar::Dot => '.',
        };

        write!(f, "{char}")
    }
}

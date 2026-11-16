use serde::Deserialize;

use crate::error::Result;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Slot {
    Token,
    Rest,
    Ext,
}

impl TryFrom<&str> for Slot {
    type Error = String;

    fn try_from(value: &str) -> std::result::Result<Self, Self::Error> {
        match value {
            "<TOKEN>" => Ok(Slot::Token),
            "<REST>" => Ok(Slot::Rest),
            "<EXT>" => Ok(Slot::Ext),
            _ => Err(value.to_string()),
        }
    }
}

#[derive(Debug)]
pub(crate) struct UnderfitLine {
    pub content: String,
    pub missing: usize,
}

#[derive(Debug, Deserialize)]
pub struct TokenizerOutput {
    pub verses: Vec<Verse>,
    #[serde(default)]
    pub meter: Option<Vec<usize>>,
}

impl TokenizerOutput {
    pub fn from_json(json: &str) -> Result<Self> {
        Ok(serde_json::from_str(json)?)
    }
}

#[derive(Debug, Deserialize)]
pub struct Verse {
    pub tokens: Vec<Token>,
}

#[derive(Debug, Deserialize)]
pub struct Token {
    pub parts: Vec<String>,
    pub operator: Option<Operator>,
}

impl Token {
    pub fn format(&self) -> String {
        match self.parts.as_slice() {
            [] => String::new(),
            [part] => escape(part),
            parts => {
                let parts = parts.iter().map(|part| escape(part)).collect::<Vec<_>>();
                format!("({})", parts.join(" "))
            }
        }
    }
}

fn escape(part: &str) -> String {
    let mut escaped = String::with_capacity(part.len());

    for ch in part.chars() {
        match ch {
            '\\' => escaped.push_str("&bls"),
            '~' => escaped.push_str("&tld"),
            '`' => escaped.push_str("&btk"),
            '(' => escaped.push_str("&lpr"),
            ')' => escaped.push_str("&rpr"),
            '@' => escaped.push_str("&atr"),
            '&' => escaped.push_str("&amp"),
            ';' => escaped.push_str("&scl"),
            _ => escaped.push(ch),
        }
    }

    escaped
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Operator {
    Space,
    Newline,
    Concat,
}

impl Operator {
    pub fn as_char(self) -> char {
        match self {
            Operator::Space => ' ',
            Operator::Newline => '\\',
            Operator::Concat => '_',
        }
    }
}

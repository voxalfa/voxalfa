use std::fmt;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("invalid tokenized lyrics: {0}")]
    Json(#[from] serde_json::Error),

    #[error(transparent)]
    Link(#[from] LinkError),
}

impl Error {
    pub fn partial(&self) -> Option<&str> {
        match self {
            Error::Json(_) => None,
            Error::Link(error) => Some(error.partial()),
        }
    }
}

#[derive(Debug, Error)]
pub enum LinkErrorKind {
    #[error("unknown slot `{slot}`")]
    UnknownSlot { slot: String },

    #[error("verse {verse}: {missing} token(s) missing")]
    Underfit { verse: usize, missing: usize },

    #[error("verse {verse}: {extra} extra token(s)")]
    Overfit { verse: usize, extra: usize },
}

#[derive(Debug)]
pub struct LinkError {
    kinds: Vec<LinkErrorKind>,
    partial: String,
}

impl LinkError {
    pub fn new(kinds: Vec<LinkErrorKind>, partial: String) -> Self {
        Self { kinds, partial }
    }

    pub fn partial(&self) -> &str {
        &self.partial
    }
}

impl fmt::Display for LinkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (index, kind) in self.kinds.iter().enumerate() {
            if index > 0 {
                writeln!(f)?;
            }

            write!(f, "{kind}")?;
        }

        Ok(())
    }
}

impl std::error::Error for LinkError {}

pub type Result<T> = std::result::Result<T, Error>;

use skrifa::raw::ReadError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("missing required header parameter '{0}'")]
    MissingHeader(&'static str),

    #[error("could not read font data: {0}")]
    Font(#[from] ReadError),

    #[error("{0}")]
    Format(#[from] std::fmt::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

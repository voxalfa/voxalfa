use skrifa::raw::ReadError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("{0}")]
    FontRead(#[from] ReadError),

    #[error("missing required header parameter field '{0}'")]
    MissingHeaderField(&'static str),
}

pub type Result<T> = std::result::Result<T, Error>;

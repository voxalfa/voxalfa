use skrifa::raw::ReadError;
use taffy::TaffyError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("{0}")]
    Taffy(#[from] TaffyError),

    #[error("{0}")]
    FontRead(#[from] ReadError),

    #[error("missing required header parameter field '{0}'")]
    MissingHeaderField(&'static str),

    #[error("{0}")]
    Format(#[from] std::fmt::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

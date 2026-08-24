use std::io;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    #[error("Validator error: {0}")]
    Validator(#[from] voxalfa_core::error::Error),

    #[error("Invalid glob pattern: {0}")]
    GlobPattern(#[from] glob::PatternError),

    #[error("Glob iteration error: {0}")]
    Glob(#[from] glob::GlobError),

    #[error("{0}")]
    Midi(#[from] voxalfa_midi::error::Error),

    #[error("{0}")]
    Svg(#[from] voxalfa_svg::error::Error),

    #[error("invalid voice {0}")]
    InvalidVoice(String),

    #[error("no matching file found")]
    NoFileMatch,
}

pub type Result<T> = std::result::Result<T, Error>;

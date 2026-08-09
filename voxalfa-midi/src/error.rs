use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("missing required header parameter field '{0}'")]
    MissingHeaderField(&'static str),

    #[error("invalid calculated MIDI key value {0} (must be between 0 and 127)")]
    InvalidMidiKey(i8),

    #[error("{0}")]
    Io(#[from] std::io::Error),

    #[error("track {0} has a duration of {1} ticks, expected {2} (fatal error)")]
    OutOfSync(usize, u32, u32),
}

pub type Result<T> = std::result::Result<T, Error>;

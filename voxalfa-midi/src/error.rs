use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConvertError {
    #[error("missing required header parameter field '{0}'")]
    MissingHeaderField(&'static str),

    #[error("invalid calculated MIDI key value {0} (must be between 0 and 127)")]
    InvalidMidiKey(i8),

    #[error("{0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, ConvertError>;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("missing required header parameter field '{0}'")]
    MissingHeaderField(&'static str),

    #[error("{0}")]
    Serialization(#[from] quick_xml::SeError),
}

pub type Result<T> = std::result::Result<T, Error>;

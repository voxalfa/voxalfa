use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("{0}")]
    LanguageError(#[from] tree_sitter::LanguageError),

    #[error("{0}")]
    QueryError(#[from] tree_sitter::QueryError),
}

pub type Result<T> = std::result::Result<T, Error>;

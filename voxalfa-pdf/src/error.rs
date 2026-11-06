use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Svg(#[from] voxalfa_svg::Error),

    #[error("invalid PDF page size")]
    PageSize,

    #[error("failed to parse SVG: {0}")]
    Parse(#[from] usvg::Error),

    #[error("failed to draw SVG on the PDF page")]
    Render,

    #[error("failed to finish PDF: {0}")]
    Pdf(#[from] krilla::error::KrillaError),
}

pub type Result<T> = std::result::Result<T, Error>;

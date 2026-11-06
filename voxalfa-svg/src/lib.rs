mod error;
mod locale;
mod model;
mod renderer;
mod resolver;
mod symbols;
mod visitor;

pub mod fonts;
pub mod options;

pub use error::{Error, Result};
pub use options::{PageSize, RenderOptions};

use voxalfa_core::output::FinalOutput;

use fonts::FontFaces;

pub fn render_pages(data: &FinalOutput) -> Result<Vec<String>> {
    render_pages_with(data, RenderOptions::default())
}

pub fn render_pages_with(data: &FinalOutput, options: RenderOptions) -> Result<Vec<String>> {
    render_pages_with_fonts(data, &options, &FontFaces::default())
}

pub fn render_pages_with_fonts(
    data: &FinalOutput,
    options: &RenderOptions,
    faces: &FontFaces,
) -> Result<Vec<String>> {
    renderer::render_pages(data, options, faces)
}

mod error;
mod fonts;
mod locale;
mod model;
mod renderer;
mod resolver;
mod symbols;
mod visitor;

pub mod options;

pub use error::{Error, Result};
pub use fonts::{FontFace, FontFaces};
pub use options::{PageSize, RenderOptions};

use voxalfa_core::output::FinalOutput;

pub fn render(data: &FinalOutput) -> Result<String> {
    render_with(data, RenderOptions::default())
}

pub fn render_with(data: &FinalOutput, options: RenderOptions) -> Result<String> {
    render_with_fonts(data, &options, &FontFaces::default())
}

pub fn render_with_fonts(
    data: &FinalOutput,
    options: &RenderOptions,
    faces: &FontFaces,
) -> Result<String> {
    renderer::render(data, options, faces)
}

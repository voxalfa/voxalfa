mod error;
mod fonts;
mod layout;
mod locale;
mod model;
mod renderer;
mod resolver;
mod symbols;
mod visitor;

pub mod options;

pub use error::{Error, Result};
pub use options::{PageSize, RenderOptions};

use voxalfa_core::output::FinalOutput;

pub fn render(data: &FinalOutput) -> Result<String> {
    renderer::render(data, &RenderOptions::default())
}

pub fn render_with(data: &FinalOutput, options: RenderOptions) -> Result<String> {
    renderer::render(data, &options)
}

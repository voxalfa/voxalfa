pub mod error;
pub mod fonts;
pub mod model;
pub mod options;
pub mod renderer;
pub mod resolver;

mod layout;
mod locale;
mod symbols;
mod visitor;

pub use error::{Error, Result};
pub use options::{PageSize, RenderOptions};
pub use renderer::Renderer;

use voxalfa_core::output::FinalOutput;

pub fn render(data: &FinalOutput) -> Result<String> {
    Renderer::default().render(data)
}

pub fn render_with(data: &FinalOutput, options: RenderOptions) -> Result<String> {
    Renderer::new(options).render(data)
}

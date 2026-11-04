//! A second-generation SVG renderer for Voxalfa partitions.
//!
//! Unlike the first renderer, this crate builds an intermediate slot model
//! from [`voxalfa_core::ir::BodyIr`]:
//!
//! * pulses are flattened across sections and grouped into measures using the
//!   time signature;
//! * every pulse exposes a logical column shape via `PulseView`, which is the
//!   unit used by lyric `span`s;
//! * lyric tokens are resolved per sub-section/voice group, so multi-lyric
//!   (`++`) systems keep their own lyrics;
//! * note positions use arbitrary offsets inside a pulse instead of a uniform
//!   grid, which keeps long notes and subdivisions compact.

pub mod error;
pub mod fonts;
pub mod model;
pub mod options;
pub mod renderer;
pub mod resolver;

pub use error::{Error, Result};
pub use options::{PageSize, RenderOptions};
pub use renderer::Renderer;

use voxalfa_core::output::FinalOutput;

/// Render a partition with the default theme.
pub fn render(data: &FinalOutput) -> Result<String> {
    Renderer::default().render(data)
}

/// Render a partition with custom options.
pub fn render_with(data: &FinalOutput, options: RenderOptions) -> Result<String> {
    Renderer::new(options).render(data)
}

use crate::{error::Result, options::RenderOptions, renderer::header::HeaderLayout};

use super::primitives::emit_text;

pub fn emit_header(
    svg: &mut String,
    header: &HeaderLayout,
    options: &RenderOptions,
    offset_y: f32,
) -> Result<()> {
    for line in &header.lines {
        emit_text(
            svg,
            line.x,
            options.page_margin.top + offset_y + line.y,
            &line.text,
            &line.class,
            line.anchor.as_str(),
        )?;
    }

    Ok(())
}

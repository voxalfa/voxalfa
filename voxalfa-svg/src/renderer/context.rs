use crate::{RenderOptions, fonts::Fonts, model::System};

#[derive(Clone, Copy)]
pub struct LayoutContext<'a, 'b> {
    pub system: &'a System<'b>,
    pub fonts: &'a Fonts,
    pub lead: f32,
    pub options: &'a RenderOptions,
}

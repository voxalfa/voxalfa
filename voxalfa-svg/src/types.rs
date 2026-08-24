use voxalfa_core::{ir::SectionIr, output::voice::VoiceSet};

#[derive(Debug, Default)]
pub struct LineSystem<'a> {
    pub internals: Vec<&'a SectionIr>,
    pub voices: Vec<VoiceSet>,
}

impl LineSystem<'_> {
    pub fn take(&mut self) -> Self {
        std::mem::take(self)
    }
}

#[derive(Debug)]
pub struct TextElement {
    pub x: f32,
    pub y: f32,
    pub content: String,
    pub class: &'static str,
}

#[derive(Debug)]
pub struct Barline {
    pub x: f32,
    pub y1: f32,
    pub y2: f32,
}

use voxalfa_core::{data_types::TimeSignature, ir::SectionIr, output::voice::VoiceSet};

#[derive(Debug)]
pub struct LineSetup {
    pub pulse_offset: usize,
}

#[derive(Debug, Default)]
pub struct LineSystem<'a> {
    pub time: TimeSignature,
    pub voices: Vec<VoiceSet>,
    pub internals: &'a [SectionIr],
}

impl LineSystem<'_> {
    pub fn take(&mut self) -> Self {
        std::mem::take(self)
    }
}

#[derive(Debug)]
pub enum Element {
    Text(TextElement),
    Barline(BarlineElement),
    Underline(UnderlineElement),
}

#[derive(Debug)]
pub struct TextElement {
    pub x: f32,
    pub y: f32,
    pub content: String,
    pub class: &'static str,
}

#[derive(Debug)]
pub struct BarlineElement {
    pub x: f32,
    pub y1: f32,
    pub y2: f32,
}

#[derive(Debug, Clone)]
pub struct UnderlineElement {
    pub x1: f32,
    pub x2: f32,
    pub y: f32,
}

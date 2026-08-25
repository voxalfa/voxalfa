use taffy::{NodeId, TaffyTree};
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
pub enum Underline {
    None,
    Partial,
    Full,
}

#[derive(Debug)]
pub struct Element {
    pub node_id: NodeId,
    pub kind: ElementKind,
}

#[derive(Debug)]
pub enum ElementKind {
    Text(TextElement),
    Barline,
}

#[derive(Debug)]
pub struct TextElement {
    pub content: String,
    pub class: &'static str,
    pub underline: Underline,
}

#[derive(Debug)]
pub struct RenderContext<'a> {
    pub tree: &'a mut TaffyTree,
    pub elements: Vec<Element>,
}

impl<'a> RenderContext<'a> {
    pub fn new(tree: &'a mut TaffyTree) -> Self {
        Self {
            tree,
            elements: Vec::new(),
        }
    }

    pub fn add_element(&mut self, node_id: NodeId, kind: ElementKind) {
        self.elements.push(Element { node_id, kind });
    }

    pub fn into_elements(self) -> Vec<Element> {
        self.elements
    }
}

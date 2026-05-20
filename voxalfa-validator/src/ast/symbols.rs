use crate::ast::types::{Key, TimeSignature, Voice};

pub type Range = tree_sitter::Range;

#[derive(Debug, Default)]
pub struct Document {
    pub header: Header,
    pub body: Body,
}

#[derive(Debug, Default)]
pub struct Header {
    pub metadata: HeaderMetadata,
    pub params: CompositionParams,
}

#[derive(Debug, Default)]
pub struct HeaderMetadata {
    pub title: Field<String>,
    pub author: Field<Vec<String>>,
    pub composer: Field<Vec<String>>,
    pub release: Field<usize>,
    pub description: Field<String>,
}

#[derive(Debug, Default)]
pub struct CompositionParams {
    pub key: Field<Key>,
    pub time: Field<TimeSignature>,
    pub bpm: Field<usize>,
    pub voices: Field<Vec<Voice>>,
}

#[derive(Debug, Default)]
pub struct SectionParams {
    pub base: CompositionParams,
    pub repeat: Field<bool>,
}

#[derive(Debug, Default)]
pub struct Body {
    pub sections: Vec<Section>,
}

#[derive(Debug, Default)]
pub struct Section {
    pub params: SectionParams,
}

#[derive(Debug, Clone, Copy)]
pub enum ValueKind {
    String,
    Integer,
    Float,
    Boolean,
    List,
    Token,
}

#[derive(Debug)]
pub struct KeyData {
    pub name: String,
    pub range: Range,
}

#[derive(Debug)]
pub struct ValueData {
    pub kind: ValueKind,
    pub range: Range,
}

#[derive(Debug)]
pub struct AssignmentData {
    pub range: Range,
    pub key: KeyData,
    pub value: ValueData,
}

#[derive(Debug)]
pub struct Assignment<T> {
    pub value: T,
    pub data: AssignmentData,
}

pub type Field<T> = Option<Assignment<T>>;

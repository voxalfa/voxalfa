use crate::{
    ast::{
        lyrics::Lyric,
        solfa::SolfaLine,
        types::{Dynamic, DynamicKind, Key, TimeSignature, Voice},
    },
    diagnostic::DiagnosticKind,
    ts_utils::{range::Range, types::AssignmentDataSource},
    validator::DocumentValidator,
};

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

pub trait FieldAssign {
    fn assign_field(&mut self, source: AssignmentDataSource, context: &mut DocumentValidator);
}

#[derive(Debug, Default)]
pub struct Document {
    pub header: Header,
    pub body: Body,
}

impl Document {
    pub fn get_voice(&self, id: usize) -> Option<Voice> {
        self.header
            .params
            .voices
            .as_ref()
            .and_then(|v| v.value.get(id))
            .copied()
    }
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

impl FieldAssign for HeaderMetadata {
    fn assign_field(&mut self, source: AssignmentDataSource, context: &mut DocumentValidator) {
        match source.data.key.name.as_str() {
            "title" => context.assign_field(source, &mut self.title),
            "author" => context.assign_field(source, &mut self.author),
            "composer" => context.assign_field(source, &mut self.composer),
            "release" => context.assign_field(source, &mut self.release),
            "description" => context.assign_field(source, &mut self.description),
            _ => {}
        }
    }
}

#[derive(Debug, Default)]
pub struct CompositionParams {
    pub key: Field<Key>,
    pub time: Field<TimeSignature>,
    pub bpm: Field<usize>,
    pub voices: Field<Vec<Voice>>,
}

impl FieldAssign for CompositionParams {
    fn assign_field(&mut self, source: AssignmentDataSource, context: &mut DocumentValidator) {
        match source.data.key.name.as_str() {
            "key" => context.assign_field(source, &mut self.key),
            "time" => context.assign_field(source, &mut self.time),
            "bpm" => context.assign_field(source, &mut self.bpm),
            "voices" => context.assign_field(source, &mut self.voices),
            _ => {
                context.report_error(
                    source.data.range,
                    DiagnosticKind::UnknownParameter(source.data.key.name.clone()),
                );
            }
        }
    }
}

#[derive(Debug, Default)]
pub struct SectionParams {
    pub base: CompositionParams,
    pub repeat: Field<bool>,
}

impl FieldAssign for SectionParams {
    fn assign_field(&mut self, source: AssignmentDataSource, context: &mut DocumentValidator) {
        match source.data.key.name.as_str() {
            "repeat" => context.assign_field(source, &mut self.repeat),
            "voices" => {
                context.report_error(
                    source.data.range,
                    DiagnosticKind::UnknownParameter(source.data.key.name.clone()),
                ); // do not allow voice override inside sections
            }
            _ => self.base.assign_field(source, context),
        }
    }
}

#[derive(Debug, Default)]
pub struct Body {
    pub sections: Vec<Section>,
}

#[derive(Debug, Default)]
pub struct Section {
    pub params: SectionParams,
    pub dynamics: Dynamics,
    pub solfa: Vec<SolfaLine>,
    pub lyrics: Vec<Lyric>,
}

#[derive(Debug, Default)]
pub struct Dynamics {
    pub value: Vec<Assignment<Dynamic>>,
}

impl FieldAssign for Dynamics {
    fn assign_field(&mut self, source: AssignmentDataSource, context: &mut DocumentValidator) {
        let name = source.data.key.name.clone();

        if let Ok(kind) = DynamicKind::try_from(name.as_str()) {
            let params = context
                .parse_node::<Vec<_>>(source.value_node)
                .unwrap_or_default();

            let expected_params = match kind {
                DynamicKind::Cre | DynamicKind::Dec => 2,
                _ => 1,
            };

            if params.len() != expected_params {
                context.report_error(
                    source.data.value.range,
                    DiagnosticKind::InvalidDynamicParams(expected_params),
                );
            } else {
                let assignment = Assignment {
                    value: Dynamic {
                        kind,
                        start: params[0],
                        end: *params.get(1).unwrap_or(&params[0]),
                    },
                    data: source.data,
                };

                self.value.push(assignment);
            }
        } else {
            context.report_error(
                source.data.range,
                DiagnosticKind::InvalidDynamic(name.clone()),
            );
        }
    }
}

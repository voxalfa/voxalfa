use crate::{
    ast::{
        symbols::{Field, FieldAssign},
        types::{Key, TimeSignature, Voice},
    },
    diagnostic::DiagnosticKind,
    ts_utils::types::AssignmentData,
    validator::DocumentValidator,
};

#[derive(Debug, Default)]
pub struct CompositionParams {
    pub key: Field<Key>,
    pub time: Field<TimeSignature>,
    pub bpm: Field<usize>,
    pub voices: Field<Vec<Voice>>,
}

impl FieldAssign for CompositionParams {
    fn assign_field(&mut self, data: AssignmentData, context: &mut DocumentValidator) {
        match data.key_name.as_str() {
            "key" => context.assign_field(data, &mut self.key),
            "time" => context.assign_field(data, &mut self.time),
            "bpm" => context.assign_field(data, &mut self.bpm),
            "voices" => context.assign_field(data, &mut self.voices),
            _ => {
                context.report_error(
                    data.full_range,
                    DiagnosticKind::UnknownParameter(data.key_name.clone()),
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
    fn assign_field(&mut self, data: AssignmentData, context: &mut DocumentValidator) {
        match data.key_name.as_str() {
            "repeat" => context.assign_field(data, &mut self.repeat),
            "voices" => {
                context.report_error(
                    data.full_range,
                    DiagnosticKind::UnknownParameter(data.key_name.clone()),
                ); // do not allow voice override inside sections
            }
            _ => self.base.assign_field(data, context),
        }
    }
}

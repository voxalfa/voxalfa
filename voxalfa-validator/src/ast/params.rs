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
    pub repeat: Field<bool>,
    pub section: bool,
}

impl FieldAssign for CompositionParams {
    fn assign_field(&mut self, data: AssignmentData, context: &mut DocumentValidator) {
        match data.key_name.as_str() {
            "key" => {
                context.assign_field(data, &mut self.key);
            }
            "time" => {
                context.assign_field(data, &mut self.time);
            }
            "bpm" => {
                context.assign_field(data, &mut self.bpm);
            }
            "voices" if !self.section => {
                context.assign_field(data, &mut self.voices);
            }
            "repeat" if self.section => {
                context.assign_field(data, &mut self.repeat);
            }
            _ => {
                context.report_error(
                    data.full_range,
                    DiagnosticKind::UnknownParameter(data.key_name.clone()),
                );
            }
        }
    }
}

use crate::{
    ast::{
        symbols::{Field, FieldAssign},
        types::{Key, TimeSignature},
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
}

impl FieldAssign for CompositionParams {
    fn assign_field(&mut self, data: AssignmentData, context: &mut DocumentValidator) {
        match data.key_name.as_str() {
            "key" => context.assign_field(data, &mut self.key),
            "time" => context.assign_field(data, &mut self.time),
            "bpm" => context.assign_field(data, &mut self.bpm),
            _ => {
                context.report_error(
                    data.full_range,
                    DiagnosticKind::UnknownParameter(data.key_name.clone()),
                );
            }
        }
    }
}

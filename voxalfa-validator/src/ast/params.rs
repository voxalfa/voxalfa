use crate::{
    ast::{
        parser::Parser,
        symbols::{Field, FieldAssign},
    },
    data_types::{Dynamic, Key, Navigation, Tempo, TimeSignature, TimedList},
    diagnostics::types::DiagnosticKind,
    ts_utils::types::AssignmentData,
};

#[derive(Debug, Default)]
pub struct InitialParams {
    pub key: Field<Key>,
    pub time: Field<TimeSignature>,
    pub tempo: Field<Tempo>,
}

impl FieldAssign for InitialParams {
    fn assign_field(&mut self, data: AssignmentData, context: &mut Parser) {
        match data.key_name.as_str() {
            "key" => context.assign_field(data, &mut self.key),
            "time" => context.assign_field(data, &mut self.time),
            "tempo" => context.assign_field(data, &mut self.tempo),
            _ => {
                context.reporter.error(
                    data.full_range,
                    DiagnosticKind::UnknownParameter(data.key_name.clone()),
                );
            }
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct SectionParams {
    pub time: Field<TimeSignature>,
    pub key: Field<TimedList<Key>>,
    pub tempo: Field<TimedList<Tempo>>,
    pub navigation: Field<TimedList<Navigation>>,
}

impl FieldAssign for SectionParams {
    fn assign_field(&mut self, data: AssignmentData, context: &mut Parser) {
        match data.key_name.as_str() {
            "key" => context.assign_field(data, &mut self.key),
            "time" => context.assign_field(data, &mut self.time),
            "tempo" => context.assign_field(data, &mut self.tempo),
            "navigation" => context.assign_field(data, &mut self.navigation),
            "dynamics" => {}
            _ => {
                context.reporter.error(
                    data.full_range,
                    DiagnosticKind::UnknownParameter(data.key_name.clone()),
                );
            }
        }
    }
}

#[derive(Debug, Default)]
pub struct SubSectionParams {
    pub dynamics: Field<TimedList<Dynamic>>,
}

impl FieldAssign for SubSectionParams {
    fn assign_field(&mut self, data: AssignmentData, context: &mut Parser) {
        if data.key_name.as_str() == "dynamics" {
            context.assign_field(data, &mut self.dynamics)
        }
    }
}

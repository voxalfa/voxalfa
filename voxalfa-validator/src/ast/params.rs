use crate::{
    ast::{
        parser::Parser,
        symbols::{Field, FieldAssign},
    },
    data_types::{Dynamic, Jump, Key, List, Mark, Tempo, TimeSignature, TimedList, Voice},
    diagnostics::types::DiagnosticKind,
    ts_utils::types::AssignmentData,
};

#[derive(Debug, Default)]
pub struct InitialParams {
    pub key: Field<Key>,
    pub time: Field<TimeSignature>,
    pub tempo: Field<Tempo>,
    pub voices: Field<List<Voice>>,
}

impl FieldAssign for InitialParams {
    fn assign_field(&mut self, data: AssignmentData, context: &mut Parser) {
        match data.key_name.as_str() {
            "key" => context.assign_field(data, &mut self.key),
            "time" => context.assign_field(data, &mut self.time),
            "tempo" => context.assign_field(data, &mut self.tempo),
            "voices" => context.assign_field(data, &mut self.voices),
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
    pub tempo: Field<Tempo>,
    pub label: Field<String>,
    pub ending: Field<usize>,
    pub key: Field<Key>,
    pub jump: Field<Jump>,
    pub mark: Field<Mark>,
}

impl SectionParams {
    pub fn has_events(&self) -> bool {
        self.key.is_some() || self.jump.is_some() || self.mark.is_some() || self.ending.is_some()
    }
}

impl FieldAssign for SectionParams {
    fn assign_field(&mut self, data: AssignmentData, context: &mut Parser) {
        match data.key_name.as_str() {
            "label" => context.assign_field(data, &mut self.label),
            "key" => context.assign_field(data, &mut self.key),
            "time" => context.assign_field(data, &mut self.time),
            "tempo" => context.assign_field(data, &mut self.tempo),
            "ending" => context.assign_field(data, &mut self.ending),
            "jump" => context.assign_field(data, &mut self.jump),
            "mark" => context.assign_field(data, &mut self.mark),
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

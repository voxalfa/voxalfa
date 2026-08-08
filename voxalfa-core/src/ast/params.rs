use crate::{
    ast::{
        parser::Parser,
        symbols::{Field, FieldAssign},
    },
    data_types::{
        Dynamic, ExtendedTempo, Jump, Key, List, Mark, StaticTempo, TimeSignature, TimedList,
        Touch, Voice,
    },
    diagnostics::types::DiagnosticKind,
    ts_utils::types::AssignmentData,
};

#[derive(Debug, Default)]
pub struct InitialParams {
    pub key: Field<Key>,
    pub time: Field<TimeSignature>,
    pub tempo: Field<StaticTempo>,
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

        if let Some(voices) = &self.voices {
            context.tree.define_voices(&voices.value);
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct SectionParams {
    pub time: Field<TimeSignature>,
    pub tempo: Field<ExtendedTempo>,
    pub label: Field<String>,
    pub ending: Field<usize>,
    pub key: Field<Key>,
    pub jump: Field<Jump>,
    pub mark: Field<Mark>,
    pub touches: Field<TimedList<Touch>>,
    pub repeat: Field<usize>,
}

impl SectionParams {
    pub fn has_events(&self) -> bool {
        self.tempo.is_some()
            || self.time.is_some()
            || self.key.is_some()
            || self.jump.is_some()
            || self.mark.is_some()
            || self.ending.is_some()
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
            "touches" => context.assign_field(data, &mut self.touches),
            "repeat" => context.assign_field(data, &mut self.repeat),
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

use std::collections::{BTreeMap, BTreeSet};

use crate::{
    ast::{dynamics::Dynamics, types::DynamicKind},
    ir::SubSectionIR,
};

pub const FLOAT_ERROR: f32 = 0.05; // allow 0.3 and 0.7 to match 1/3 and 2/3

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Timestamp {
    pub pulse_index: usize,
    pub note_index: usize,
    pub duration: usize,
    pub factor: usize,
}

#[derive(Debug, Default)]
pub struct Timeline {
    pub dynamics: BTreeMap<Timestamp, DynamicEvent>,
}

#[derive(Debug)]
pub struct DynamicEvent {
    pub dynamic: DynamicKind,
    pub kind: DynamicEventKind,
}

impl DynamicEvent {
    pub fn start(value: DynamicKind) -> Self {
        Self {
            dynamic: value,
            kind: DynamicEventKind::Start,
        }
    }

    pub fn end(value: DynamicKind) -> Self {
        Self {
            dynamic: value,
            kind: DynamicEventKind::End,
        }
    }
}

#[derive(Debug)]
pub enum DynamicEventKind {
    Start,
    End,
}

#[derive(Debug, Default)]
pub struct DynamicsBuffer {
    absolute_offset: usize,
    relative_offset: usize,
    results: Vec<(Timestamp, DynamicEvent)>,
    processed: BTreeSet<usize>,
}

impl DynamicsBuffer {
    pub fn init_section(&mut self) {
        self.relative_offset = 0;
        self.processed.clear();
    }

    pub fn has_processed(&self, id: usize) -> bool {
        self.processed.contains(&id)
    }

    pub fn drain(&mut self) -> impl Iterator<Item = (Timestamp, DynamicEvent)> {
        self.results.drain(..)
    }

    pub fn is_empty(&self) -> bool {
        self.results.is_empty()
    }

    pub fn process(&mut self, sections: &SubSectionIR, dynamics: &Dynamics) {
        if dynamics.value.is_empty() {
            return;
        }

        for view in &sections.views {
            let mut ellapsed = self.relative_offset as f32;

            for (note_index, &duration) in view.durations.iter().enumerate() {
                let params = DynamicMatchParams {
                    note_index,
                    note_start: ellapsed,
                    note_end: ellapsed + (duration as f32 / view.factor as f32),
                    duration,
                    factor: view.factor,
                };

                self.match_dynamics(&params, &dynamics);

                ellapsed = params.note_end;
            }

            self.absolute_offset += 1;
            self.relative_offset += 1;
        }
    }

    fn match_dynamics(&mut self, params: &DynamicMatchParams, dynamics: &Dynamics) {
        for (id, dynamic) in dynamics.value.iter().enumerate() {
            let start_diff = (dynamic.value.start - params.note_start).abs();
            let end_diff = (dynamic.value.end - params.note_end).abs();

            if start_diff < FLOAT_ERROR {
                let timestamp = self.create_timestamp(params);
                let event = DynamicEvent::start(dynamic.value.kind);

                self.results.push((timestamp, event));

                if dynamic.value.is_mark() {
                    self.processed.insert(id);
                }
            }

            if dynamic.value.is_range() && end_diff < FLOAT_ERROR {
                let timestamp = self.create_timestamp(params);
                let event = DynamicEvent::end(dynamic.value.kind);

                self.results.push((timestamp, event));
                self.processed.insert(id);
            }
        }
    }

    fn create_timestamp(&self, params: &DynamicMatchParams) -> Timestamp {
        Timestamp {
            pulse_index: self.absolute_offset,
            note_index: params.note_index,
            duration: params.duration,
            factor: params.factor,
        }
    }
}

#[derive(Debug)]
struct DynamicMatchParams {
    note_index: usize,
    note_start: f32,
    note_end: f32,
    duration: usize,
    factor: usize,
}

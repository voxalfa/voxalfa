use std::collections::HashMap;

use crate::{
    ast::{dynamics::Dynamics, types::DynamicKind},
    ir::SubSectionIR,
};

pub const FLOAT_ERROR: f32 = 0.05; // allow 0.3 and 0.7 to match 1/3 and 2/3

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Timestamp {
    pub pulse_index: usize,
    pub note_index: usize,
}

impl From<&DynamicMatchParams> for Timestamp {
    fn from(value: &DynamicMatchParams) -> Self {
        Self {
            pulse_index: value.pulse_index,
            note_index: value.note_index,
        }
    }
}

pub type SubSectonId = usize;

#[derive(Debug, Default)]
pub struct TimelineMap {
    timelines: HashMap<SubSectonId, Timeline>,
}

impl TimelineMap {
    pub fn extend_from_buffer(&mut self, buffer: &mut DynamicsBuffer) {
        for partial in buffer.results.drain(..) {
            self.timelines
                .entry(partial.sub_id)
                .or_default()
                .add_event(partial.timestamp, partial.event);
        }
    }

    pub fn get(&mut self, sub_id: SubSectonId) -> Option<&Timeline> {
        self.timelines.get(&sub_id)
    }
}

#[derive(Debug, Default)]
pub struct Timeline {
    events: Vec<(Timestamp, DynamicEvent)>,
}

impl Timeline {
    pub fn add_event(&mut self, timestamp: Timestamp, event: DynamicEvent) {
        self.events.push((timestamp, event));
    }

    pub fn get_event(&self, timestamp: Timestamp) -> Option<DynamicEvent> {
        self.events
            .binary_search_by(|(ts, _)| ts.cmp(&timestamp))
            .ok()
            .map(|idx| self.events[idx].1)
    }
}

#[derive(Debug, Clone, Copy)]
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

#[derive(Debug, Clone, Copy)]
pub enum DynamicEventKind {
    Start,
    End,
}

#[derive(Debug, Default)]
pub struct DynamicsBuffer {
    offset: usize,
    results: Vec<BufferedEvent>,
    processed: Vec<usize>,
}

impl DynamicsBuffer {
    pub fn init_section(&mut self) {
        self.offset = 0;
        self.processed.clear();
    }

    pub fn is_empty(&self) -> bool {
        self.results.is_empty()
    }

    pub fn has_processed(&self, id: usize) -> bool {
        self.processed.contains(&id)
    }

    pub fn process(&mut self, sub_section: &SubSectionIR, dynamics: &Dynamics) {
        if dynamics.value.is_empty() {
            return;
        }

        for (pulse_index, view) in sub_section.views.iter().enumerate() {
            let mut ellapsed = self.offset as f32;

            for (note_index, &duration) in view.durations.iter().enumerate() {
                let params = DynamicMatchParams {
                    note_start: ellapsed,
                    note_end: ellapsed + (duration as f32 / view.factor as f32),
                    pulse_index,
                    note_index,
                };

                self.match_dynamics(sub_section.sid, &params, &dynamics);

                ellapsed = params.note_end;
            }

            self.offset += 1;
        }
    }

    fn match_dynamics(
        &mut self,
        sub_id: SubSectonId,
        params: &DynamicMatchParams,
        dynamics: &Dynamics,
    ) {
        for (id, dynamic) in dynamics.value.iter().enumerate() {
            let start_diff = (dynamic.value.start - params.note_start).abs();
            let end_diff = (dynamic.value.end - params.note_end).abs();

            if start_diff < FLOAT_ERROR {
                let timestamp = Timestamp::from(params);
                let event = DynamicEvent::start(dynamic.value.kind);

                self.push_event(sub_id, timestamp, event);

                if dynamic.value.is_mark() {
                    self.processed.push(id);
                }
            }

            if dynamic.value.is_range() && end_diff < FLOAT_ERROR {
                let timestamp = Timestamp::from(params);
                let event = DynamicEvent::end(dynamic.value.kind);

                self.push_event(sub_id, timestamp, event);
                self.processed.push(id);
            }
        }
    }

    fn push_event(&mut self, sub_id: SubSectonId, timestamp: Timestamp, event: DynamicEvent) {
        // TODO: watch out repeat flows using seg and DS
        self.results.push(BufferedEvent {
            sub_id,
            timestamp,
            event,
        });
    }
}

#[derive(Debug)]
struct DynamicMatchParams {
    note_start: f32,
    note_end: f32,
    pulse_index: usize,
    note_index: usize,
}

#[derive(Debug)]
struct BufferedEvent {
    sub_id: SubSectonId,
    timestamp: Timestamp,
    event: DynamicEvent,
}

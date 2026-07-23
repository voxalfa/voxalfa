use crate::{
    ast::dynamics::Dynamics,
    ir::SubSectionIR,
    output::{DynamicEvent, SubSectonId, Timestamp},
};

pub const FLOAT_ERROR: f32 = 0.05; // allow 0.3 and 0.7 to match 1/3 and 2/3

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

    pub fn drain(&mut self) -> impl Iterator<Item = BufferedEvent> {
        self.results.drain(..)
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

                self.match_dynamics(sub_section.sid, dynamics, &params);

                ellapsed = params.note_end;
            }

            self.offset += 1;
        }
    }

    fn match_dynamics(
        &mut self,
        sub_id: SubSectonId,
        dynamics: &Dynamics,
        params: &DynamicMatchParams,
    ) {
        for (id, dynamic) in dynamics.value.iter().enumerate() {
            let start_diff = (dynamic.value.start - params.note_start).abs();
            let end_diff = (dynamic.value.end - params.note_end).abs();

            if start_diff < FLOAT_ERROR {
                let timestamp = Timestamp::new(params.pulse_index, params.note_index);
                let event = DynamicEvent::start(dynamic.value.kind);

                self.push_event(sub_id, timestamp, event);

                if dynamic.value.is_mark() {
                    self.processed.push(id);
                }
            }

            if dynamic.value.is_range() && end_diff < FLOAT_ERROR {
                let timestamp = Timestamp::new(params.pulse_index, params.note_index);
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
pub struct BufferedEvent {
    pub sub_id: SubSectonId,
    pub timestamp: Timestamp,
    pub event: DynamicEvent,
}

#[derive(Debug)]
struct DynamicMatchParams {
    note_start: f32,
    note_end: f32,
    pulse_index: usize,
    note_index: usize,
}

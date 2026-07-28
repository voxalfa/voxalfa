use crate::{
    data_types::TimedList,
    event::{Event, SubSectonId, Timestamp, ToEventKind},
    ir::SubSectionIR,
};

pub const FLOAT_ERROR: f32 = 0.05; // allow 0.3 and 0.7 to match 1/3 and 2/3

#[derive(Debug, Default)]
pub struct EventBuffer {
    offset: usize,
    results: Vec<BufferedEvent>,
    processed: Vec<usize>,
}

impl EventBuffer {
    pub fn init_section(&mut self) {
        self.offset = 0;
        self.processed.clear();
    }

    pub fn has_processed(&self, id: usize) -> bool {
        self.processed.contains(&id)
    }

    pub fn drain(&mut self) -> impl Iterator<Item = BufferedEvent> {
        self.results.drain(..)
    }

    pub fn process<T: ToEventKind + std::fmt::Debug>(
        &mut self,
        sub_section: &SubSectionIR,
        events: &TimedList<T>,
    ) {
        if events.is_empty() {
            return;
        }

        for (pulse_index, view) in sub_section.views.iter().enumerate() {
            let mut ellapsed = self.offset as f32;

            for (note_index, &duration) in view.durations.iter().enumerate() {
                let params = EventMatchParams {
                    note_start: ellapsed,
                    note_end: ellapsed + (duration as f32 / view.factor as f32),
                    pulse_index,
                    note_index,
                };

                self.match_event(sub_section.sid, events, &params);

                ellapsed = params.note_end;
            }

            self.offset += 1;
        }
    }

    fn match_event<T: ToEventKind>(
        &mut self,
        sub_id: SubSectonId,
        events: &TimedList<T>,
        params: &EventMatchParams,
    ) {
        for (id, current) in events.iter().enumerate() {
            let start_diff = (current.value.start - params.note_start).abs();
            let end_diff = (current.value.end.unwrap_or_default() - params.note_end).abs();

            if start_diff < FLOAT_ERROR {
                let timestamp = Timestamp::new(params.pulse_index, params.note_index);
                let event = Event::start(current.value.value.to_event_kind());

                self.push_event(sub_id, timestamp, event);

                if current.value.end.is_none() {
                    self.processed.push(id);
                }
            }

            if current.value.end.is_some() && end_diff < FLOAT_ERROR {
                let timestamp = Timestamp::new(params.pulse_index, params.note_index);
                let event = Event::end(current.value.value.to_event_kind());

                self.push_event(sub_id, timestamp, event);
                self.processed.push(id);
            }
        }
    }

    fn push_event(&mut self, sub_id: SubSectonId, timestamp: Timestamp, event: Event) {
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
    pub event: Event,
}

#[derive(Debug)]
struct EventMatchParams {
    note_start: f32,
    note_end: f32,
    pulse_index: usize,
    note_index: usize,
}

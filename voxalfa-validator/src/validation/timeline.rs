use crate::{
    data_types::TimedList,
    event::{Event, EventKind, SubSectonId, Timestamp, ToEventKind},
    ir::SubSectionIR,
};

pub const FLOAT_ERROR: f32 = 0.05; // allow 0.3 and 0.7 to match 1/3 and 2/3

#[derive(Debug, Default)]
pub struct EventBuffer {
    offset: usize,
    results: Vec<BufferedEvent>,
    processed: Vec<(EventKind, usize)>,
}

impl EventBuffer {
    pub fn init_section(&mut self) {
        self.offset = 0;
        self.processed.clear();
    }

    pub fn has_processed(&self, kind: EventKind, id: usize) -> bool {
        self.processed.contains(&(kind, id))
    }

    pub fn add_offset(&mut self, offset: usize) {
        self.offset += offset;
    }

    pub fn drain(&mut self) -> impl Iterator<Item = BufferedEvent> {
        self.results.drain(..)
    }

    pub fn process<T: ToEventKind>(&mut self, sub_section: &SubSectionIR, events: &TimedList<T>) {
        if events.is_empty() {
            return;
        }

        let mut ellapsed = self.offset as f32;

        for (pulse_index, view) in sub_section.views.iter().enumerate() {
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

            ellapsed += 1.;
        }
    }

    fn match_event<T: ToEventKind>(
        &mut self,
        sub_id: SubSectonId,
        events: &TimedList<T>,
        params: &EventMatchParams,
    ) {
        for (id, symbol) in events.iter().enumerate() {
            let current = &symbol.value;
            let kind = current.value.to_event_kind();

            if self.has_processed(kind, id) {
                continue;
            }

            if current.value.is_range() {
                let end = current.end.unwrap_or_default();

                if self.check_eq(end, params.note_start) || self.check_eq(end, params.note_end) {
                    self.mark_event(kind, id);
                }
            }

            let timestamp = if self.check_eq(current.start, params.note_start) {
                Timestamp::start(params.pulse_index, params.note_index)
            } else if self.check_eq(current.start, params.note_end) {
                Timestamp::end(params.pulse_index, params.note_index)
            } else {
                continue;
            };

            let span = current.end.map(|end| end - current.start);
            let event = Event::new(kind, span);

            self.push_event(sub_id, timestamp, event);

            if !current.value.is_range() {
                self.mark_event(kind, id);
            }
        }
    }

    fn check_eq(&self, lhs: f32, rhs: f32) -> bool {
        (lhs - rhs).abs() < FLOAT_ERROR
    }

    fn mark_event(&mut self, kind: EventKind, id: usize) {
        self.processed.push((kind, id));
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

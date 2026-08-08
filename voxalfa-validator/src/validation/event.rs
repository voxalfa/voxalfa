use crate::{
    data_types::TimedList,
    ir::{SectionIr, SubSectionIr},
    output::event::{Event, EventKind, SubSectonId, Timestamp, ToEventKind, get_note_ticks},
};

pub const FLOAT_ERROR: f32 = 0.05; // allow 0.3 and 0.7 to match 1/3 and 2/3

#[derive(Debug, Default)]
pub struct EventBuffer {
    elapsed: usize,
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
        self.elapsed += get_note_ticks(offset, 1);
    }

    pub fn drain(&mut self) -> impl Iterator<Item = BufferedEvent> {
        self.results.drain(..)
    }

    pub fn process_section_events(&mut self, section: &SectionIr) {
        if !section.params.has_events() {
            return;
        }

        for sub_section in &section.items {
            let sid = sub_section.sid;
            let start_timestamp = self.elapsed;
            let end_timestamp = self.elapsed + sub_section.get_ticks();

            for event in section.start_events() {
                self.push_event(sid, start_timestamp, event);
            }

            for event in section.end_events() {
                self.push_event(sid, end_timestamp, event);
            }
        }
    }

    pub fn process_local_events<T: ToEventKind>(
        &mut self,
        sub_section: &SubSectionIr,
        events: &TimedList<T>,
    ) {
        if events.is_empty() {
            return;
        }

        let mut relative_offset = self.offset as f32;
        let mut elapsed = self.elapsed;

        for view in &sub_section.views {
            for &duration in &view.durations {
                let note_ticks = get_note_ticks(duration, view.factor);
                let note_start = relative_offset;
                let note_end = relative_offset + (duration as f32 / view.factor as f32);

                let params = EventMatchParams {
                    note_start,
                    note_end,
                    note_ticks,
                    elapsed,
                };

                self.match_event(sub_section.sid, events, &params);

                relative_offset = note_end;
                elapsed += note_ticks;
            }
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
                let timestamp = current.end.and_then(|e| self.get_timestamp(e, params));

                if let Some(timestamp) = timestamp {
                    self.push_event(sub_id, timestamp, Event::new(kind));
                    self.mark_event(kind, id);
                    continue;
                }
            }

            if let Some(timestamp) = self.get_timestamp(current.start, params) {
                self.push_event(sub_id, timestamp, Event::new(kind));

                if !current.value.is_range() {
                    self.mark_event(kind, id);
                }
            };
        }
    }

    fn get_timestamp(&self, value: f32, params: &EventMatchParams) -> Option<Timestamp> {
        if self.check_eq(value, params.note_start) {
            Some(params.elapsed)
        } else if self.check_eq(value, params.note_end) {
            Some(params.elapsed + params.note_ticks)
        } else {
            None
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
    note_ticks: usize,
    elapsed: usize,
}

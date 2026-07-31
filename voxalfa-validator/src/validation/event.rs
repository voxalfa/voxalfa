use crate::{
    data_types::TimedList,
    ir::{SectionIR, SubSectionIR},
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

    pub fn process_section_events(&mut self, section: &SectionIR) {
        if !section.params.has_events() {
            return;
        }

        for sub_section in &section.items {
            let sid = sub_section.sid;

            if let Some(key) = &section.params.key {
                self.push_event(sid, self.elapsed, Event::simple(key.value));
            }

            if let Some(mark) = &section.params.mark {
                self.push_event(sid, self.elapsed, Event::simple(mark.value));
            }

            if let Some(jump) = &section.params.jump {
                self.push_event(
                    sid,
                    self.elapsed + sub_section.get_ticks(),
                    Event::simple(jump.value),
                );
            }

            if let Some(ending) = &section.params.ending {
                self.push_event(
                    sid,
                    self.elapsed,
                    Event::new(EventKind::EndingStart(ending.value), None),
                );

                self.push_event(
                    sid,
                    self.elapsed + sub_section.get_ticks(),
                    Event::new(EventKind::EndingEnd(ending.value), None),
                );
            }
        }
    }

    pub fn process_local_events<T: ToEventKind>(
        &mut self,
        sub_section: &SubSectionIR,
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

            relative_offset += 1.;
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
                params.elapsed
            } else if self.check_eq(current.start, params.note_end) {
                params.elapsed + params.note_ticks
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
    note_ticks: usize,
    elapsed: usize,
}

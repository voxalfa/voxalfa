use crate::{
    ast::symbols::SymbolId,
    data_types::TimedList,
    ir::{BodyIr, PulseView, SectionIr, SubSectionIr},
    output::event::{
        EndingEvent, Event, EventAnchor, EventKind, EventRole, SubSectonId, TimedPlacement,
        Timestamp, ToEventKind, get_note_ticks,
    },
};

pub const FLOAT_ERROR: f32 = 0.05; // allow 0.3 and 0.7 to match 1/3 and 2/3

#[derive(Debug, Default)]
pub struct EventBuffer {
    elapsed: usize,
    offset: usize,
    results: Vec<BufferedEvent>,
    processed: Vec<SymbolId>,
    last_ending: Option<u8>,
}

impl EventBuffer {
    pub fn new(body: &BodyIr) -> Self {
        let last_ending = body.sections.iter().rev().find_map(|section| {
            section
                .params
                .ending
                .as_ref()
                .map(|ending| ending.value as u8)
        });

        Self {
            last_ending,
            ..Default::default()
        }
    }

    pub fn init_section(&mut self) {
        self.offset = 0;
        self.processed.clear();
    }

    pub fn init_sub_section(&mut self) {
        self.processed.clear();
    }

    pub fn has_processed(&self, sid: SymbolId) -> bool {
        self.processed.contains(&sid)
    }

    pub fn add_offset(&mut self, offset: usize) {
        self.offset += offset;
        self.elapsed += get_note_ticks(offset as u8, 1);
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

    pub fn process_local_events<T: ToEventKind + TimedPlacement>(
        &mut self,
        sub_section: &SubSectionIr,
        events: &TimedList<T>,
    ) {
        if events.is_empty() {
            return;
        }

        let last_index = self.section_last_index(&sub_section.views);
        let mut relative_offset = self.offset as f32;
        let mut elapsed = self.elapsed;

        for view in &sub_section.views {
            for &duration in &view.durations {
                let note_ticks = get_note_ticks(duration, view.factor);
                let note_start = relative_offset;
                let note_end = relative_offset + (duration as f32 / view.factor as f32);

                let params = EventMatchParams {
                    note_start,
                    note_ticks,
                    elapsed,
                };

                self.match_event(sub_section.sid, events, &params, last_index);

                relative_offset = note_end;
                elapsed += note_ticks;
            }
        }
    }

    fn match_event<T: ToEventKind + TimedPlacement>(
        &mut self,
        sub_id: SubSectonId,
        events: &TimedList<T>,
        params: &EventMatchParams,
        last_index: f32,
    ) {
        for symbol in events.iter() {
            let current = &symbol.value;
            let kind = current.value.to_event_kind();
            let anchor = current.value.anchor();
            let index = if current.implicit {
                current.value.implicit_index(last_index)
            } else {
                current.start
            };

            if self.has_processed(symbol.sid) {
                continue;
            }

            if current.value.is_range() {
                if let Some(timestamp) = self.get_timestamp(index, params, anchor) {
                    self.push_event(
                        sub_id,
                        timestamp,
                        Event {
                            kind,
                            role: EventRole::Start,
                            anchor,
                        },
                    );
                }

                let end = if current.implicit {
                    current.value.implicit_range_end(last_index)
                } else {
                    current.end
                };
                if let Some(end) = end
                    && let Some(timestamp) = self.get_timestamp(end, params, EventAnchor::Before)
                {
                    self.push_event(
                        sub_id,
                        timestamp,
                        Event {
                            kind,
                            role: EventRole::End,
                            anchor: EventAnchor::Before,
                        },
                    );
                    self.mark_event(symbol.sid);
                }

                continue;
            }

            if let Some(timestamp) = self.get_timestamp(index, params, anchor) {
                self.push_event(
                    sub_id,
                    timestamp,
                    Event {
                        kind,
                        role: EventRole::Point,
                        anchor,
                    },
                );
                self.mark_event(symbol.sid);
            };
        }
    }

    fn get_timestamp(
        &self,
        value: f32,
        params: &EventMatchParams,
        anchor: EventAnchor,
    ) -> Option<Timestamp> {
        if self.check_eq(value, params.note_start) {
            Some(match anchor {
                EventAnchor::Before => params.elapsed,
                EventAnchor::After => params.elapsed + params.note_ticks,
            })
        } else {
            None
        }
    }

    fn section_last_index(&self, views: &[PulseView]) -> f32 {
        let mut index = self.offset as f32;
        let mut last = index;

        for view in views {
            for &duration in &view.durations {
                last = index;
                index += duration as f32 / view.factor as f32;
            }
        }

        last
    }

    fn check_eq(&self, lhs: f32, rhs: f32) -> bool {
        (lhs - rhs).abs() < FLOAT_ERROR
    }

    fn mark_event(&mut self, sid: SymbolId) {
        self.processed.push(sid);
    }

    fn push_event(&mut self, sub_id: SubSectonId, timestamp: Timestamp, event: Event) {
        let event = match event {
            Event {
                kind: EventKind::Ending(ending),
                role,
                anchor,
            } => Event {
                kind: EventKind::Ending(EndingEvent {
                    is_last: self.last_ending == Some(ending.id),
                    ..ending
                }),
                role,
                anchor,
            },
            event => event,
        };

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
    note_ticks: usize,
    elapsed: usize,
}

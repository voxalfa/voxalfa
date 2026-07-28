use std::collections::HashMap;

use crate::{data_types::Dynamic, validation::timeline::EventBuffer};

pub type SubSectonId = usize;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Timestamp {
    pub pulse_index: usize,
    pub note_index: usize,
}

impl Timestamp {
    pub fn new(pulse_index: usize, note_index: usize) -> Self {
        Self {
            pulse_index,
            note_index,
        }
    }
}

#[derive(Debug, Default)]
pub struct TimelineMap {
    timelines: HashMap<SubSectonId, Timeline>,
}

impl TimelineMap {
    pub fn extend_from_buffer(&mut self, buffer: &mut EventBuffer) {
        for partial in buffer.drain() {
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
    events: Vec<(Timestamp, Event)>,
}

impl Timeline {
    pub fn add_event(&mut self, timestamp: Timestamp, event: Event) {
        self.events.push((timestamp, event));
    }

    pub fn get_event(&self, timestamp: Timestamp) -> Option<&Event> {
        self.events
            .binary_search_by(|(ts, _)| ts.cmp(&timestamp))
            .ok()
            .map(|idx| &self.events[idx].1)
    }
}

#[derive(Debug)]
pub struct Event {
    pub dispatch: EventDispatch,
    pub kind: EventKind,
}

impl Event {
    pub fn start(kind: EventKind) -> Self {
        Self {
            dispatch: EventDispatch::Start,
            kind,
        }
    }

    pub fn end(kind: EventKind) -> Self {
        Self {
            dispatch: EventDispatch::End,
            kind,
        }
    }
}

#[derive(Debug)]
pub enum EventKind {
    Dynamic(Dynamic),
}

#[derive(Debug, Clone, Copy)]
pub enum EventDispatch {
    Start,
    End,
}

pub trait ToEventKind {
    fn to_event_kind(&self) -> EventKind;
}

impl ToEventKind for Dynamic {
    fn to_event_kind(&self) -> EventKind {
        EventKind::Dynamic(*self)
    }
}

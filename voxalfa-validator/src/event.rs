use std::collections::HashMap;

use crate::{
    data_types::{Dynamic, Jump, Key, Mark},
    validation::timeline::EventBuffer,
};

pub type SubSectonId = usize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TimestampKind {
    Start,
    End,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Timestamp {
    pub pulse_index: usize,
    pub note_index: usize,
    pub kind: TimestampKind,
}

impl Timestamp {
    pub fn start(pulse_index: usize, note_index: usize) -> Self {
        Self {
            pulse_index,
            note_index,
            kind: TimestampKind::Start,
        }
    }

    pub fn end(pulse_index: usize, note_index: usize) -> Self {
        Self {
            pulse_index,
            note_index,
            kind: TimestampKind::End,
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

    pub fn get(&self, sub_id: SubSectonId) -> Option<&Timeline> {
        self.timelines.get(&sub_id)
    }

    pub fn get_mut(&mut self, sub_id: SubSectonId) -> &mut Timeline {
        self.timelines.entry(sub_id).or_default()
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

    pub fn get_events(&self, timestamp: Timestamp) -> impl Iterator<Item = &Event> {
        self.events
            .iter()
            .filter(move |(ts, _)| *ts == timestamp)
            .map(|(_, ev)| ev)
    }
}

#[derive(Debug)]
pub struct Event {
    pub kind: EventKind,
    pub span: Option<f32>,
}

impl Event {
    pub fn new(kind: EventKind, span: Option<f32>) -> Self {
        Self { kind, span }
    }

    pub fn simple<T: ToEventKind>(value: T) -> Self {
        Self {
            kind: value.to_event_kind(),
            span: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EventKind {
    Dynamic(Dynamic),
    Key(Key),
    Jump(Jump),
    Mark(Mark),
    EndingStart(usize),
    EndingEnd(usize),
}

#[derive(Debug, Clone, Copy)]
pub enum EventDispatch {
    Start,
    End,
}

pub trait ToEventKind {
    fn to_event_kind(&self) -> EventKind;

    fn is_range(&self) -> bool {
        false
    }
}

impl ToEventKind for Dynamic {
    fn to_event_kind(&self) -> EventKind {
        EventKind::Dynamic(*self)
    }

    fn is_range(&self) -> bool {
        matches!(self, Dynamic::Cre | Dynamic::Dec)
    }
}

impl ToEventKind for Key {
    fn to_event_kind(&self) -> EventKind {
        EventKind::Key(*self)
    }
}

impl ToEventKind for Jump {
    fn to_event_kind(&self) -> EventKind {
        EventKind::Jump(*self)
    }
}

impl ToEventKind for Mark {
    fn to_event_kind(&self) -> EventKind {
        EventKind::Mark(*self)
    }
}

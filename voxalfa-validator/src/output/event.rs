use std::collections::HashMap;

use crate::{
    data_types::{Dynamic, Jump, Key, Mark, Touch},
    validation::event::EventBuffer,
};

pub type SubSectonId = usize;
pub type Timestamp = usize;
pub type NoteId = usize;

pub const TICK_PER_WHOLE_NOTE: usize = 960;

pub fn get_note_ticks(numerator: usize, denominator: usize) -> Timestamp {
    (TICK_PER_WHOLE_NOTE * numerator) / denominator
}

type PartialTimeline = Vec<(Timestamp, Event)>;

#[derive(Debug, Default)]
pub struct TimelineMap {
    timelines: HashMap<SubSectonId, PartialTimeline>,
}

impl TimelineMap {
    pub fn extend_from_buffer(&mut self, buffer: &mut EventBuffer) {
        for partial in buffer.drain() {
            self.timelines
                .entry(partial.sub_id)
                .or_default()
                .push((partial.timestamp, partial.event));
        }
    }

    pub fn get(&self, sub_id: SubSectonId) -> Option<&PartialTimeline> {
        self.timelines.get(&sub_id)
    }

    pub fn get_mut(&mut self, sub_id: SubSectonId) -> &mut PartialTimeline {
        self.timelines.entry(sub_id).or_default()
    }
}

#[derive(Debug, Default)]
pub struct NoteTimeline {
    events: Vec<(NoteId, Event)>,
}

impl NoteTimeline {
    pub fn add_event(&mut self, note_id: NoteId, event: Event) {
        self.events.push((note_id, event));
    }

    pub fn get_events(&self, note_id: NoteId) -> impl Iterator<Item = &Event> {
        self.events
            .iter()
            .filter(move |(ts, _)| *ts == note_id)
            .map(|(_, ev)| ev)
    }
}

#[derive(Debug, Clone)]
pub struct Event {
    pub kind: EventKind,
}

impl Event {
    pub fn new(kind: EventKind) -> Self {
        Self { kind }
    }

    pub fn with<T: ToEventKind>(value: T) -> Self {
        Self {
            kind: value.to_event_kind(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EventKind {
    Dynamic(Dynamic),
    Key(Key),
    Jump(Jump),
    Mark(Mark),
    Touch(Touch),
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

impl ToEventKind for Touch {
    fn to_event_kind(&self) -> EventKind {
        EventKind::Touch(*self)
    }
}

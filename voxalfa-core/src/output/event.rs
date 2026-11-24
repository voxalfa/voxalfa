use std::collections::HashMap;

use crate::{
    data_types::{
        Dynamic, ExtendedTempo, Jump, Key, Mark, ProgressiveTempo, StaticTempo, TimeSignature,
        Touch,
    },
    validation::event::EventBuffer,
};

pub type SubSectonId = usize;
pub type Timestamp = usize;
pub type NoteId = usize;

pub const TICK_PER_PULSE: usize = 960;

pub fn get_note_ticks(numerator: u8, denominator: u8) -> Timestamp {
    (TICK_PER_PULSE * numerator as usize) / denominator as usize
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
    pub role: EventRole,
    pub anchor: EventAnchor,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventAnchor {
    Before,
    After,
}

pub trait TimedPlacement {
    fn anchor(&self) -> EventAnchor;

    fn implicit_index(&self, _last_index: f32) -> f32 {
        0.0
    }

    fn implicit_range_end(&self, _last_index: f32) -> Option<f32> {
        None
    }
}

impl TimedPlacement for Dynamic {
    fn anchor(&self) -> EventAnchor {
        EventAnchor::Before
    }
}

impl TimedPlacement for Touch {
    fn anchor(&self) -> EventAnchor {
        EventAnchor::Before
    }
}

impl TimedPlacement for Key {
    fn anchor(&self) -> EventAnchor {
        EventAnchor::Before
    }
}

impl TimedPlacement for ExtendedTempo {
    fn anchor(&self) -> EventAnchor {
        EventAnchor::Before
    }

    fn implicit_range_end(&self, last_index: f32) -> Option<f32> {
        matches!(self, ExtendedTempo::Progressive(_)).then_some(last_index)
    }
}

impl TimedPlacement for Mark {
    fn anchor(&self) -> EventAnchor {
        match self {
            Mark::Segno | Mark::Coda => EventAnchor::Before,
            Mark::Fine | Mark::ToCoda => EventAnchor::After,
        }
    }

    fn implicit_index(&self, last_index: f32) -> f32 {
        match self {
            Mark::Segno | Mark::Coda => 0.0,
            Mark::Fine | Mark::ToCoda => last_index,
        }
    }
}

impl TimedPlacement for JumpEvent {
    fn anchor(&self) -> EventAnchor {
        EventAnchor::After
    }

    fn implicit_index(&self, last_index: f32) -> f32 {
        last_index
    }
}

impl Event {
    pub fn new(kind: EventKind) -> Self {
        Self {
            kind,
            role: EventRole::Point,
            anchor: EventAnchor::Before,
        }
    }

    pub fn start(kind: EventKind) -> Self {
        Self {
            kind,
            role: EventRole::Start,
            anchor: EventAnchor::Before,
        }
    }

    pub fn end(kind: EventKind) -> Self {
        Self {
            kind,
            role: EventRole::End,
            anchor: EventAnchor::After,
        }
    }

    pub fn after(kind: EventKind) -> Self {
        Self {
            kind,
            role: EventRole::Point,
            anchor: EventAnchor::After,
        }
    }

    pub fn with<T: ToEventKind>(value: T) -> Self {
        Self::new(value.to_event_kind())
    }

    pub fn is_tail_event(&self) -> bool {
        self.anchor == EventAnchor::After
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EventKind {
    Dynamic(Dynamic),
    Key(Key),
    TimeSignature(TimeSignature),
    Tempo(StaticTempo),
    ProgressiveTempo(ProgressiveTempo),
    Jump(JumpEvent),
    Mark(Mark),
    Touch(Touch),
    Ending(EndingEvent),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EndingEvent {
    pub id: u8,
    pub is_last: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct JumpEvent {
    pub kind: Jump,
    pub repeat: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventRole {
    Point,
    Start,
    End,
}

pub trait ToEventKind {
    fn to_event_kind(&self) -> EventKind;

    fn is_range(&self) -> bool {
        false
    }
}

impl ToEventKind for TimeSignature {
    fn to_event_kind(&self) -> EventKind {
        EventKind::TimeSignature(*self)
    }
}

impl ToEventKind for Dynamic {
    fn to_event_kind(&self) -> EventKind {
        EventKind::Dynamic(*self)
    }

    fn is_range(&self) -> bool {
        matches!(self, Dynamic::Cresc | Dynamic::Decresc | Dynamic::Dim)
    }
}

impl ToEventKind for Key {
    fn to_event_kind(&self) -> EventKind {
        EventKind::Key(*self)
    }
}

impl ToEventKind for StaticTempo {
    fn to_event_kind(&self) -> EventKind {
        EventKind::Tempo(*self)
    }
}

impl ToEventKind for ExtendedTempo {
    fn to_event_kind(&self) -> EventKind {
        match self {
            ExtendedTempo::Static(tempo) => EventKind::Tempo(*tempo),
            ExtendedTempo::Progressive(tempo) => EventKind::ProgressiveTempo(*tempo),
        }
    }

    fn is_range(&self) -> bool {
        matches!(self, ExtendedTempo::Progressive(_))
    }
}

impl ToEventKind for JumpEvent {
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

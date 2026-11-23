use std::collections::HashMap;

use crate::{
    data_types::{
        BaseKey, Dynamic, Jump, Key, KeyAccidental, Mark, ProgressiveTempo, StaticTempo,
        TimeSignature, Touch,
    },
    output::{
        dynamics::{DynamicState, DynamicTransition, DynamicTransitionKind},
        event::{Event, EventAnchor, EventKind, EventRole, JumpEvent, NoteTimeline},
        tempo::{TempoState, TempoTransition},
    },
};

#[derive(Debug)]
pub struct TimelineEvaluator {
    index: usize,
    jump: Option<usize>,
    segno: usize,
    endings_jump: HashMap<u8, usize>,
    jump_table: HashMap<usize, u8>,
    target_mark: Option<Mark>,
    waited_event: Option<EventKind>,
    pending_touch: Option<Touch>,
    abort: bool,
    pub params: PlaybackParams,
    pub dynamic: DynamicState,
    pub tempo: TempoState,
}

impl TimelineEvaluator {
    pub fn new(params: PlaybackParams) -> Self {
        Self {
            params,
            index: 0,
            endings_jump: HashMap::new(),
            jump_table: HashMap::new(),
            segno: 0,
            jump: None,
            target_mark: None,
            waited_event: None,
            abort: false,
            pending_touch: None,
            tempo: TempoState::default(),
            dynamic: DynamicState::default(),
        }
    }

    pub fn index(&self) -> usize {
        self.index
    }

    pub fn jump(&mut self) -> bool {
        if let Some(pointer) = self.jump.take() {
            self.index = pointer;
            true
        } else {
            false
        }
    }

    pub fn is_waiting(&self) -> bool {
        self.waited_event.is_some()
    }

    pub fn done(&self) -> bool {
        self.abort
    }

    pub fn step(&mut self) {
        self.index += 1;
    }

    pub fn handle_events(&mut self, timeline: &NoteTimeline, anchor: EventAnchor) {
        let events = timeline
            .get_events(self.index)
            .filter(|event| event.anchor == anchor);

        for event in events {
            self.handle_event(event);
        }
    }

    pub fn take_pending_touch(&mut self) -> Option<Touch> {
        self.pending_touch.take()
    }

    pub fn poll_dynamic_update(&mut self) -> Option<DynamicTransition> {
        self.dynamic.ended.take()
    }

    pub fn poll_tempo_update(&mut self) -> Option<TempoTransition> {
        if self.tempo.update {
            self.tempo.update = false;
            self.tempo.transition.take()
        } else {
            None
        }
    }

    pub fn handle_event(&mut self, event: &Event) {
        self.waited_event.take_if(|e| *e == event.kind);

        match event.kind {
            EventKind::Key(key) => self.params.key = key,
            EventKind::Tempo(tempo) => self.params.tempo = tempo,
            EventKind::TimeSignature(time) => self.params.time = time,
            EventKind::Touch(touch) => self.pending_touch = Some(touch),
            EventKind::Mark(mark) => self.handle_mark_event(mark),
            EventKind::Jump(jump) => self.handle_jump_event(jump),

            EventKind::Dynamic(dynamic) => match event.role {
                EventRole::Point => self.dynamic.current = dynamic,
                EventRole::Start => self.handle_dynamic_start(dynamic),
                EventRole::End => self.handle_dynamic_end(dynamic),
            },

            EventKind::ProgressiveTempo(kind) => match event.role {
                EventRole::Start => self.handle_tempo_start(kind),
                _ => self.handle_tempo_end(kind),
            },

            EventKind::Ending(ending) => match event.role {
                EventRole::Start if let Some(address) = self.endings_jump.get(&ending.id) => {
                    self.index = *address;
                }
                EventRole::End => {
                    let address = if event.anchor == EventAnchor::After {
                        self.index + 1
                    } else {
                        self.index
                    };

                    self.endings_jump.insert(ending.id, address);
                }
                _ => {}
            },
        }
    }

    fn handle_mark_event(&mut self, mark: Mark) {
        match mark {
            Mark::Segno => self.segno = self.index,
            Mark::Fine if self.target_mark == mark.into() => self.abort = true,
            Mark::ToCoda if self.target_mark == mark.into() => {
                self.waited_event = Some(EventKind::Mark(Mark::Coda));
            }
            _ => {}
        }
    }

    fn handle_jump_event(&mut self, jump: JumpEvent) {
        let entry = self.jump_table.entry(self.index).or_insert(jump.repeat);

        if *entry > 0 {
            let address = match jump.kind {
                Jump::DS | Jump::DSC | Jump::DSF => self.segno,
                _ => 0,
            };

            self.jump = Some(address);
            self.target_mark = jump.kind.target_mark();

            *entry -= 1;
        }
    }

    fn handle_dynamic_start(&mut self, dynamic: Dynamic) {
        match dynamic {
            Dynamic::Cresc => self.start_dynamic_transition(DynamicTransitionKind::Inc),
            Dynamic::Decresc | Dynamic::Dim => {
                self.start_dynamic_transition(DynamicTransitionKind::Dec)
            }
            _ => self.dynamic.current = dynamic,
        }
    }

    fn start_dynamic_transition(&mut self, kind: DynamicTransitionKind) {
        if self.dynamic.transition.is_none() {
            self.dynamic.transition = Some(DynamicTransition {
                level: self.dynamic.current,
                kind,
            });
        }
    }

    fn handle_dynamic_end(&mut self, dynamic: Dynamic) {
        let kind = match dynamic {
            Dynamic::Cresc => DynamicTransitionKind::Inc,
            _ => DynamicTransitionKind::Dec,
        };

        let Some(transition) = self.dynamic.transition.take() else {
            return;
        };

        // handle adjacent dynamic edge case
        if transition.kind != kind {
            self.dynamic.transition = Some(transition);
        } else {
            if self.dynamic.current == transition.level {
                self.dynamic.current = transition.target(self.dynamic.current);
            }

            self.dynamic.ended = Some(transition);
        }
    }

    fn handle_tempo_start(&mut self, kind: ProgressiveTempo) {
        self.tempo.transition = Some(TempoTransition {
            kind,
            initial_value: self.params.tempo.bpm(),
        })
    }

    fn handle_tempo_end(&mut self, kind: ProgressiveTempo) {
        if self
            .tempo
            .transition
            .as_ref()
            .is_some_and(|t| t.kind == kind)
        {
            self.tempo.update = true;
        }
    }
}

#[derive(Debug, Clone)]
pub struct PlaybackParams {
    pub key: Key,
    pub time: TimeSignature,
    pub tempo: StaticTempo,
}

impl PlaybackParams {
    pub fn new(key: Key, time: TimeSignature, tempo: StaticTempo) -> Self {
        Self { key, time, tempo }
    }

    pub fn dummy() -> Self {
        Self {
            key: Key {
                base: BaseKey::C,
                accidental: KeyAccidental::Neutral,
            },
            time: TimeSignature { top: 4, bottom: 4 },
            tempo: StaticTempo::Moderato,
        }
    }
}

use std::collections::HashMap;

use midly::{
    MetaMessage, MidiMessage, Track, TrackEvent, TrackEventKind,
    num::{u4, u7, u28},
};
use voxalfa_validator::{
    ast::solfa::Note,
    data_types::{Dynamic, Jump, Key, Mark, Touch, Voice},
    output::{
        event::{Event, EventKind, JumpEvent, NoteTimeline},
        voice::NoteContext,
    },
};

use crate::{
    BASE_MIDI_KEY, MAX_PAUSE, PPQN,
    dynamics::{DynamicState, DynamicTransition, DynamicTransitionKind},
    error::{ConvertError, Result},
};

#[derive(Debug)]
pub struct VoiceTask {
    index: usize,
    jump: Option<usize>,
    channel: u4,
    params: PlaybackParams,
    voice: Voice,
    track: Track<'static>,
    active_note: Option<u7>,
    play_ticks: u32,
    rest_ticks: u32,
    pending_touch: Option<Touch>,
    pending_notes: Vec<PendingNote>,
    dynamic: DynamicState,
    slur: bool,
    segno: usize,
    endings_jump: HashMap<u8, usize>,
    jump_table: HashMap<usize, u8>,
    target_mark: Option<Mark>,
    waited_event: Option<EventKind>,
    abort: bool,
}

impl VoiceTask {
    pub fn new(id: usize, voice: Voice, params: PlaybackParams) -> Self {
        Self {
            voice,
            params,
            index: 0,
            channel: u4::from(id as u8),
            track: Track::new(),
            active_note: None,
            play_ticks: 0,
            rest_ticks: 0,
            pending_touch: None,
            pending_notes: Vec::new(),
            dynamic: DynamicState::default(),
            endings_jump: HashMap::new(),
            jump_table: HashMap::new(),
            segno: 0,
            slur: false,
            jump: None,
            target_mark: None,
            waited_event: None,
            abort: false,
        }
    }

    pub fn get_midi_note(&self, note: Note) -> Result<u7> {
        let result = BASE_MIDI_KEY + self.params.key.offset() + note.offset() + self.voice.offset();

        if !(0..=127).contains(&result) {
            Err(ConvertError::InvalidMidiKey(result))
        } else {
            Ok(u7::from(result as u8))
        }
    }

    pub fn handle_note(&mut self, note: Note, ctx: &NoteContext<'_>) -> Result<()> {
        let touch = self.pending_touch.take();

        if self.dynamic.transition.is_some() {
            let key = self.get_midi_note(note)?;
            let duration = self.get_midi_note_ticks(ctx);

            self.pending_notes.push(PendingNote {
                key,
                duration,
                touch,
            });
        } else {
            self.apply_note_context(ctx);
            self.handle_active_note();

            let raw_duration = self.get_midi_note_ticks(ctx);
            let (play_ticks, rest_ticks) = self.get_touch_ticks(raw_duration, touch);

            let midi_note = self.get_midi_note(note)?;
            let mut velocity = self.get_velocity(self.dynamic.current);

            if touch == Some(Touch::Accent) {
                velocity = u8::max(velocity.as_int() + 20, 127).into();
            }

            self.note_on(midi_note, velocity);

            self.active_note = Some(midi_note);
            self.play_ticks = play_ticks;
            self.rest_ticks = rest_ticks;
        }

        Ok(())
    }

    pub fn handle_pause(&mut self, ctx: &NoteContext<'_>) {
        self.apply_note_context(ctx);
        self.handle_active_note();

        self.rest_ticks += self.get_midi_note_ticks(ctx);
    }

    pub fn prolongate(&mut self, ctx: &NoteContext<'_>) {
        let ticks = self.get_midi_note_ticks(ctx);

        if self.dynamic.transition.is_some()
            && let Some(last) = self.pending_notes.last_mut()
        {
            last.duration += ticks;
        } else {
            self.apply_note_context(ctx);
            self.play_ticks += ticks;
        }
    }

    pub fn finalize(mut self) -> Track<'static> {
        self.handle_active_note();

        self.track.push(TrackEvent {
            delta: u28::from(self.play_ticks),
            kind: TrackEventKind::Meta(MetaMessage::EndOfTrack),
        });

        self.track
    }

    pub fn index(&self) -> usize {
        self.index
    }

    pub fn jump(&mut self) -> bool {
        if let Some(pointer) = self.jump.take() {
            self.index = pointer;
            self.handle_active_note();
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

    pub fn handle_events(&mut self, timeline: &NoteTimeline) {
        let events = timeline.get_events(self.index);

        for event in events {
            self.handle_event(event);
        }

        if self.dynamic.update
            && let Some(transition) = self.dynamic.transition.take()
        {
            self.dynamic.update = false;
            self.handle_pending_notes(transition);
        }
    }

    pub fn handle_pending_events(&mut self, timeline: &NoteTimeline) {
        self.handle_events(timeline);
        self.jump();
    }

    pub fn handle_event(&mut self, event: &Event) {
        self.waited_event.take_if(|e| *e == event.kind);

        match event.kind {
            EventKind::Key(key) => self.params.key = key,
            EventKind::Touch(touch) => self.pending_touch = Some(touch),
            EventKind::Mark(mark) => self.handle_mark_event(mark),
            EventKind::Jump(jump) => self.handle_jump_event(jump),
            EventKind::Dynamic(dynamic) => self.handle_dynamic_event(dynamic),

            EventKind::EndingStart(id) => {
                if let Some(address) = self.endings_jump.get(&id) {
                    self.index = *address;
                }
            }

            EventKind::EndingEnd(id) => {
                self.endings_jump.insert(id, self.index + 1);
            }
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

    fn handle_dynamic_event(&mut self, dynamic: Dynamic) {
        match dynamic {
            Dynamic::Cre | Dynamic::Dec if self.dynamic.transition.is_some() => {
                self.dynamic.update = true;
            }

            Dynamic::Cre => {
                self.dynamic.transition = Some(DynamicTransition {
                    level: self.dynamic.current,
                    kind: DynamicTransitionKind::Cre,
                });
            }

            Dynamic::Dec => {
                self.dynamic.transition = Some(DynamicTransition {
                    level: self.dynamic.current,
                    kind: DynamicTransitionKind::Dec,
                });
            }

            _ => {
                self.dynamic.current = dynamic;
            }
        }
    }

    fn handle_active_note(&mut self) {
        if let Some(last_note) = self.active_note.take() {
            self.note_off(last_note);
            self.play_ticks = self.rest_ticks;
            self.rest_ticks = 0;
        } else {
            self.play_ticks += self.rest_ticks;
            self.rest_ticks = 0;
        }
    }

    fn handle_pending_notes(&mut self, transition: DynamicTransition) {
        let notes = std::mem::take(&mut self.pending_notes);

        if notes.is_empty() {
            return;
        }

        let start_vel = u8::from(self.get_velocity(transition.level)) as f32;
        let end_vel = u8::from(self.get_target_velocity(transition)) as f32;

        let total_ticks = notes.iter().map(|n| n.duration).sum::<u32>();
        let mut elapsed_ticks: u32 = 0;

        for note in notes {
            let progress = if total_ticks > 0 {
                elapsed_ticks as f32 / total_ticks as f32
            } else {
                0.0
            };

            let vel_val = (start_vel + (end_vel - start_vel) * progress)
                .round()
                .clamp(0.0, 127.0) as u8;

            let velocity = u7::from(vel_val);
            let (play_ticks, rest_ticks) = self.get_touch_ticks(note.duration, note.touch);

            self.handle_active_note();
            self.note_on(note.key, velocity);

            self.active_note = Some(note.key);
            self.play_ticks = play_ticks;
            self.rest_ticks = rest_ticks;

            elapsed_ticks += note.duration;
        }
    }

    fn apply_note_context(&mut self, ctx: &NoteContext<'_>) {
        if ctx.note.underline.left {
            self.slur = true;
        }

        if ctx.note.underline.right {
            self.slur = false;
        }
    }

    fn note_on(&mut self, key: u7, vel: u7) {
        self.track.push(TrackEvent {
            delta: u28::from(self.play_ticks),
            kind: TrackEventKind::Midi {
                channel: self.channel,
                message: MidiMessage::NoteOn { key, vel },
            },
        });
    }

    fn note_off(&mut self, note: u7) {
        self.track.push(TrackEvent {
            delta: u28::from(self.play_ticks),
            kind: TrackEventKind::Midi {
                channel: self.channel,
                message: MidiMessage::NoteOff {
                    key: note,
                    vel: u7::from(0),
                },
            },
        });
    }

    fn get_midi_note_ticks(&self, ctx: &NoteContext<'_>) -> u32 {
        let denominator = ctx.factor as u32;
        let numerator = ctx.note.duration as u32;
        ((PPQN as u32 * numerator) / denominator) / (4 / self.params.quarter_unit)
    }

    fn get_touch_ticks(&self, duration: u32, touch: Option<Touch>) -> (u32, u32) {
        match touch {
            Some(Touch::Staccato) => {
                let play_ticks = duration / 2;
                let rest_ticks = duration - play_ticks;
                (play_ticks, rest_ticks)
            }
            Some(Touch::Fermata) => (duration + (duration / 2), 0),
            _ if self.slur => (duration, 0),
            _ => {
                let rest_ticks = (duration / 10).min(MAX_PAUSE);
                let play_ticks = duration - rest_ticks;

                (play_ticks, rest_ticks)
            }
        }
    }

    fn get_velocity(&self, dynamic: Dynamic) -> u7 {
        match dynamic {
            Dynamic::PPP => u7::from(16),
            Dynamic::PP => u7::from(32),
            Dynamic::P => u7::from(48),
            Dynamic::MP => u7::from(64),
            Dynamic::MF => u7::from(80),
            Dynamic::F => u7::from(96),
            Dynamic::FF => u7::from(112),
            Dynamic::FFF => u7::from(127),
            _ => unreachable!("invalid velicity access"),
        }
    }

    fn get_target_velocity(&self, transition: DynamicTransition) -> u7 {
        let dynamic = match transition.kind {
            _ if transition.level == self.dynamic.current => Some(self.dynamic.current),
            DynamicTransitionKind::Cre => transition.level.get_next(),
            DynamicTransitionKind::Dec => transition.level.get_prev(),
        };

        self.get_velocity(dynamic.unwrap_or(transition.level))
    }
}

#[derive(Debug, Clone)]
pub struct PlaybackParams {
    pub key: Key,
    pub quarter_unit: u32,
}

impl PlaybackParams {
    pub fn new(key: Key, quarter_unit: usize) -> Self {
        Self {
            key,
            quarter_unit: quarter_unit as u32,
        }
    }
}

#[derive(Debug)]
struct PendingNote {
    key: u7,
    duration: u32,
    touch: Option<Touch>,
}

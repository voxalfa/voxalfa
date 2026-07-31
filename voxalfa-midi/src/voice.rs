use std::collections::HashMap;

use midly::{
    MetaMessage, MidiMessage, Track, TrackEvent, TrackEventKind,
    num::{u4, u7, u28},
};
use voxalfa_validator::{
    ast::solfa::Note,
    data_types::{Key, Voice},
    output::{
        NoteContext,
        event::{Event, EventKind, NoteTimeline},
    },
};

use crate::{
    BASE_MIDI_KEY, DEFAULT_VELOCITY, PPQ,
    error::{ConvertError, Result},
};

#[derive(Debug)]
pub struct VoiceTask {
    index: usize,
    jump: Option<usize>,
    channel: u4,
    key: Key,
    voice: Voice,
    track: Track<'static>,
    active_note: Option<u7>,
    pending_ticks: u32,
    velocity: u7,
    slur: bool,
    marks: [usize; 3],
    endings_jump: HashMap<usize, usize>,
    jump_table: HashMap<usize, u8>,
}

impl VoiceTask {
    pub fn new(id: usize, voice: Voice, key: Key) -> Self {
        Self {
            key,
            voice,
            index: 0,
            channel: u4::from(id as u8),
            track: Track::new(),
            active_note: None,
            pending_ticks: 0,
            velocity: u7::from(DEFAULT_VELOCITY),
            slur: false,
            endings_jump: HashMap::new(),
            jump_table: HashMap::new(),
            marks: [0; 3],
            jump: None,
        }
    }

    pub fn get_midi_note(&self, note: Note) -> Result<u7> {
        let result = BASE_MIDI_KEY + self.key.offset() + note.offset() + self.voice.offset();

        if !(0..=127).contains(&result) {
            Err(ConvertError::InvalidMidiKey(result))
        } else {
            Ok(u7::from(result as u8))
        }
    }

    pub fn handle_note(&mut self, note: Note, ctx: &NoteContext<'_>) -> Result<()> {
        self.handle_note_context(ctx);
        self.handle_active_note(0);

        let midi_note = self.get_midi_note(note)?;

        self.note_on(midi_note);
        self.active_note = Some(midi_note);
        self.pending_ticks = self.get_midi_note_ticks(ctx);

        Ok(())
    }

    pub fn handle_pause(&mut self, ctx: &NoteContext<'_>) {
        self.handle_note_context(ctx);

        let ticks = self.get_midi_note_ticks(ctx);

        if self.handle_active_note(ticks) {
            self.pending_ticks = ticks;
        } else {
            self.pending_ticks += ticks;
        }
    }

    pub fn prolongate(&mut self, ctx: &NoteContext<'_>) {
        self.handle_note_context(ctx);
        self.pending_ticks += self.get_midi_note_ticks(ctx);
    }

    pub fn finalize(mut self) -> Track<'static> {
        self.handle_active_note(0);

        self.track.push(TrackEvent {
            delta: u28::from(self.pending_ticks),
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
            self.handle_active_note(0);
            true
        } else {
            false
        }
    }

    pub fn step(&mut self) {
        self.index += 1;
    }

    pub fn handle_events(&mut self, timeline: &NoteTimeline) {
        let events = timeline.get_events(self.index);

        for event in events {
            self.handle_event(event);
        }
    }

    pub fn handle_pending_events(&mut self, timeline: &NoteTimeline) {
        self.handle_events(timeline);
        self.jump();
    }

    pub fn handle_event(&mut self, event: &Event) {
        match event.kind {
            EventKind::Key(key) => {
                self.key = key;
            }

            EventKind::Dynamic(_dynamic) => {}

            EventKind::Mark(mark) => {
                self.marks[mark as usize] = self.index;
            }

            EventKind::Jump(jump) => {
                if self.jump_table.get(&self.index).is_none() {
                    self.jump = Some(self.marks[jump.mark() as usize]);
                    self.jump_table.insert(self.index, 0); // TODO: actual repeat count
                }
            }

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

    fn handle_active_note(&mut self, ticks: u32) -> bool {
        if let Some(last_note) = self.active_note.take() {
            self.note_off(last_note);
            self.pending_ticks = ticks;

            true
        } else {
            false
        }
    }

    // FIXME: figure out a way to apply slurs?
    fn handle_note_context(&mut self, ctx: &NoteContext<'_>) {
        if ctx.note.underline.left {
            self.slur = true;
        }

        if ctx.note.underline.right {
            self.slur = false;
        }
    }

    fn note_on(&mut self, note: u7) {
        self.track.push(TrackEvent {
            delta: u28::from(self.pending_ticks),
            kind: TrackEventKind::Midi {
                channel: self.channel,
                message: MidiMessage::NoteOn {
                    key: note,
                    vel: self.velocity,
                },
            },
        });
    }

    fn note_off(&mut self, note: u7) {
        self.track.push(TrackEvent {
            delta: u28::from(self.pending_ticks),
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
        let ticks = (PPQ as u32 * numerator) / denominator;

        ticks
    }
}

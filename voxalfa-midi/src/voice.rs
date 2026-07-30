use std::collections::HashMap;

use midly::{
    MetaMessage, MidiMessage, Track, TrackEvent, TrackEventKind,
    num::{u4, u7, u28},
};
use voxalfa_validator::{
    ast::solfa::Note,
    data_types::{Key, Voice},
    event::{Event, EventKind},
    output::NoteContext,
};

use crate::{
    BASE_MIDI_KEY, DEFAULT_VELOCITY, PPQ,
    error::{ConvertError, Result},
};

#[derive(Debug)]
pub struct VoiceTask<'a> {
    pointer: usize,
    jump: Option<usize>,
    channel: u4,
    key: Key,
    voice: Voice,
    track: Track<'static>,
    active_note: Option<u7>,
    pending_ticks: u32,
    velocity: u7,
    slur: bool,
    pending_event: Vec<&'a Event>,
    marks: [usize; 3],
    endings_jump: HashMap<usize, usize>,
}

impl<'a> VoiceTask<'a> {
    pub fn new(id: usize, voice: Voice, key: Key) -> Self {
        Self {
            key,
            voice,
            pointer: 0,
            channel: u4::from(id as u8),
            track: Track::new(),
            active_note: None,
            pending_ticks: 0,
            pending_event: Vec::new(),
            velocity: u7::from(DEFAULT_VELOCITY),
            slur: false,
            endings_jump: HashMap::new(),
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

    pub fn handle_note(&mut self, note: Note, params: NoteParams) -> Result<()> {
        self.handle_note_params(&params);
        self.handle_active_note(0);

        let midi_note = self.get_midi_note(note)?;

        self.note_on(midi_note);
        self.active_note = Some(midi_note);
        self.pending_ticks = params.ticks;

        Ok(())
    }

    pub fn handle_pause(&mut self, params: NoteParams) {
        self.handle_note_params(&params);

        if self.handle_active_note(params.ticks) {
            self.pending_ticks = params.ticks;
        } else {
            self.pending_ticks += params.ticks;
        }
    }

    pub fn prolongate(&mut self, params: NoteParams) {
        self.handle_note_params(&params);
        self.pending_ticks += params.ticks;
    }

    pub fn handle_events(&mut self, events: impl Iterator<Item = &'a Event>) {
        for event in events {
            self.handle_event(event);
        }
    }

    pub fn schedule_events(&mut self, events: impl Iterator<Item = &'a Event>) {
        self.pending_event.extend(events);
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
        self.pointer
    }

    pub fn step(&mut self) {
        if let Some(pointer) = self.jump.take() {
            self.pointer = pointer;
        } else {
            self.pointer += 1;
        }
    }

    fn handle_event(&mut self, event: &'a Event) {
        match event.kind {
            EventKind::Key(key) => self.key = key,

            EventKind::Dynamic(_dynamic) => {}

            EventKind::Mark(mark) => {
                self.marks[mark as usize] = self.pointer;
            }

            EventKind::Jump(jump) => {
                self.jump = Some(self.marks[jump.mark() as usize]);
            }

            EventKind::EndingStart(id) => {
                if let Some(address) = self.endings_jump.get(&id) {
                    self.jump = Some(*address);
                }
            }

            EventKind::EndingEnd(id) => {
                self.endings_jump.insert(id, self.pointer + 1);
            }
        }
    }

    fn handle_active_note(&mut self, ticks: u32) -> bool {
        while let Some(event) = self.pending_event.pop() {
            self.handle_event(event);
        }

        if let Some(last_note) = self.active_note.take() {
            self.note_off(last_note);
            self.pending_ticks = ticks;

            true
        } else {
            false
        }
    }

    // FIXME: figure out a way to apply slurs?
    fn handle_note_params(&mut self, params: &NoteParams) {
        if params.slur_start {
            self.slur = true;
        }

        if params.slur_end {
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
}

#[derive(Debug)]
pub struct NoteParams {
    pub ticks: u32,
    pub slur_start: bool,
    pub slur_end: bool,
}

impl NoteParams {
    pub fn new(ctx: &NoteContext) -> Self {
        let denominator = ctx.pulse.factor as u32;
        let numerator = ctx.note.duration as u32;
        let ticks = (PPQ as u32 * numerator) / denominator;

        Self {
            ticks,
            slur_start: ctx.note.underline.left,
            slur_end: ctx.note.underline.right,
        }
    }
}

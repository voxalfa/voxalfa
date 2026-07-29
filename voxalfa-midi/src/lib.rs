pub mod error;

use std::path::PathBuf;

use midly::{
    Format, Header, MetaMessage, MidiMessage, Smf, Timing, Track, TrackEvent, TrackEventKind,
    num::{u4, u7, u15, u24, u28},
};
use voxalfa_validator::{
    ast::{solfa::Note, symbols::SymbolRef},
    data_types::{Key, TimeSignature, Voice},
    event::{Event, EventKind},
    ir::solfa::PulseColumnKind,
    output::{FinalOutput, NoteContext},
};

use crate::error::{ConvertError, Result};

pub const PPQ: u16 = 480;
pub const BASE_MIDI_KEY: i8 = 60; // middle C
pub const DEFAULT_VELOCITY: u8 = 90;

#[allow(unused)]
#[derive(Debug)]
pub struct Converter<'a> {
    source: &'a FinalOutput,
}

impl<'a> Converter<'a> {
    pub fn new(source: &'a FinalOutput) -> Self {
        Self { source }
    }

    pub fn convert(&mut self, output_path: PathBuf) -> Result<()> {
        let mut smf = Smf::new(Header::new(
            Format::Parallel,
            Timing::Metrical(u15::from(PPQ)),
        ));

        let params = &self.source.header.params;

        let key = self.get_header_param("key", params.key.as_ref())?;
        let voices = self.get_header_param("voices", params.voices.as_ref())?;
        let time = self.get_header_param("time", params.time.as_ref())?;
        let tempo = self.get_header_param("tempo", params.tempo.as_ref())?;

        // TODO: mid track tempo/time signature change
        let tempo_track = self.create_tempo_track(tempo.bpm(), time.top, time.bottom);

        smf.tracks.push(tempo_track);

        for (id, voice) in voices.iter().enumerate() {
            let track = self.process_voice(id, voice.value, *key)?;
            smf.tracks.push(track);
        }

        smf.save(output_path)?;

        Ok(())
    }

    fn get_header_param<T>(
        &self,
        name: &'static str,
        param: Option<&'a SymbolRef<T>>,
    ) -> Result<&'a T> {
        param
            .ok_or(ConvertError::MissingHeaderField(name))
            .map(|f| &f.value)
    }

    fn process_voice(&mut self, id: usize, voice: Voice, key: Key) -> Result<Track<'static>> {
        let mut task = ConverterTask::new(id, voice, key);
        let voice_line = self.source.build_voice_line(voice);

        // TODO: event handling and branching
        for ctx in &voice_line {
            let params = NoteParams::new(ctx);

            if let Some(start_event) = ctx.start_event() {
                task.handle_event(start_event);
            }

            match ctx.note.kind {
                PulseColumnKind::Note(note) => task.handle_note(note, params)?,
                PulseColumnKind::EmptyNote => task.handle_pause(params),
                PulseColumnKind::ProlongedNote(_) => task.prolongate(params),
            }

            if let Some(end_event) = ctx.end_event() {
                task.schedule_event(end_event);
            }
        }

        Ok(task.finalize())
    }

    fn create_tempo_track(
        &self,
        bpm: usize,
        numerator: usize,
        denominator: usize,
    ) -> Track<'static> {
        let mut track = Track::new();

        let denom_exponent = (denominator as f32).log2() as u8;
        let tempo = self.bpm_to_uspq(bpm);

        track.push(TrackEvent {
            delta: u28::from(0),
            kind: TrackEventKind::Meta(MetaMessage::TimeSignature(
                numerator as u8,
                denom_exponent,
                24,
                8,
            )),
        });

        track.push(TrackEvent {
            delta: u28::from(0),
            kind: TrackEventKind::Meta(MetaMessage::Tempo(tempo)),
        });

        track.push(TrackEvent {
            delta: u28::from(0),
            kind: TrackEventKind::Meta(MetaMessage::EndOfTrack),
        });

        track
    }

    fn bpm_to_uspq(&self, bpm: usize) -> u24 {
        let us_per_quarter = (60_000_000.0 / bpm as f64).round() as u32;
        u24::from(us_per_quarter & 0x00FF_FFFF)
    }
}

#[derive(Debug)]
pub struct TrackParams {
    pub key: Key,
    pub time: TimeSignature,
    pub bpm: u16,
}

#[derive(Debug)]
pub struct ConverterTask<'a> {
    channel: u4,
    key: Key,
    voice: Voice,
    track: Track<'static>,
    active_note: Option<u7>,
    pending_ticks: u32,
    _play_count: u8,
    velocity: u7,
    slur: bool,
    pending_event: Option<&'a Event>,
}

impl<'a> ConverterTask<'a> {
    pub fn new(id: usize, voice: Voice, key: Key) -> Self {
        Self {
            key,
            voice,
            channel: u4::from(id as u8),
            track: Track::new(),
            active_note: None,
            pending_ticks: 0,
            pending_event: None,
            velocity: u7::from(DEFAULT_VELOCITY),
            _play_count: 0,
            slur: false,
        }
    }

    pub fn get_midi_note(&self, note: Note) -> Result<u7> {
        let result =
            BASE_MIDI_KEY + self.key.offset() + note.offset() + 12 * self.voice.octave_offset();

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

    pub fn handle_event(&mut self, event: &Event) {
        match event.kind {
            EventKind::Key(key) => self.key = key,
            EventKind::Dynamic(_dynamic) => {}
            EventKind::Navigation(_navigation) => {}
            EventKind::Tempo(_tempo) => {}
        }
    }

    pub fn schedule_event(&mut self, event: &'a Event) {
        self.pending_event = Some(event);
    }

    pub fn finalize(mut self) -> Track<'static> {
        self.handle_active_note(0);

        self.track.push(TrackEvent {
            delta: u28::from(self.pending_ticks),
            kind: TrackEventKind::Meta(MetaMessage::EndOfTrack),
        });

        self.track
    }

    fn handle_active_note(&mut self, ticks: u32) -> bool {
        if let Some(last_note) = self.active_note.take() {
            self.note_off(last_note);
            self.pending_ticks = ticks;

            if let Some(event) = self.pending_event.take() {
                self.handle_event(event);
            }

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

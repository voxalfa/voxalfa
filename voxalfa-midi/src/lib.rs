pub mod error;

mod tempo;
mod voice;

use std::path::PathBuf;

use midly::{Format, Header, Smf, Timing, Track, num::u15};
use voxalfa_validator::{
    ast::symbols::SymbolRef,
    data_types::{Key, Tempo, TimeSignature, Voice},
    ir::solfa::PulseColumnKind,
    output::FinalOutput,
};

use crate::{
    error::{ConvertError, Result},
    tempo::TempoTask,
    voice::{NoteParams, VoiceTask},
};

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
        let tempo_track = self.create_tempo_track(tempo, time);

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
        let mut task = VoiceTask::new(id, voice, key);
        let voice_line = self.source.build_voice_line(voice);

        while let Some(ctx) = voice_line.get(task.index()) {
            let params = NoteParams::new(ctx);

            if let Some(events) = ctx.start_event() {
                task.handle_events(events);
            }

            match ctx.note.kind {
                PulseColumnKind::Note(note) => task.handle_note(note, params)?,
                PulseColumnKind::EmptyNote => task.handle_pause(params),
                PulseColumnKind::ProlongedNote(_) => task.prolongate(params),
            }

            if let Some(events) = ctx.end_event() {
                task.schedule_events(events);
            }

            task.step();
        }

        Ok(task.finalize())
    }

    fn create_tempo_track(
        &self,
        initial_tempo: &Tempo,
        initial_time: &TimeSignature,
    ) -> Track<'static> {
        let mut task = TempoTask::new(initial_tempo, initial_time);

        for section in &self.source.ir.sections {
            let ticks = PPQ as u32 * section.items[0].views.len() as u32;

            if let Some(time) = &section.params.time {
                task.handle_signature(&time.value);
            }

            if let Some(tempo) = &section.params.tempo {
                task.handle_tempo(&tempo.value);
            }

            task.handle_ticks(ticks);
        }

        task.finalize()
    }
}

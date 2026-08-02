pub mod error;

mod tempo;
mod voice;

#[cfg(test)]
mod tests;

use midly::{Format, Header, Smf, Timing, Track, num::u15};
use voxalfa_validator::{
    ast::symbols::SymbolRef,
    data_types::{Key, Tempo, TimeSignature, Voice},
    output::{
        FinalOutput,
        evaluator::{PlaybackParams, TimelineEvaluator},
    },
};

use crate::{
    error::{ConvertError, Result},
    tempo::TempoTask,
    voice::VoiceTask,
};

pub const PPQN: u16 = 480;
pub const MAX_PAUSE: u32 = (PPQN as u32) / 20;
pub const BASE_MIDI_KEY: i8 = 60; // middle C

#[allow(unused)]
#[derive(Debug)]
pub struct Converter<'a> {
    source: &'a FinalOutput,
}

impl<'a> Converter<'a> {
    pub fn new(source: &'a FinalOutput) -> Self {
        Self { source }
    }

    pub fn convert(mut self) -> Result<Smf<'static>> {
        let mut smf = Smf::new(Header::new(
            Format::Parallel,
            Timing::Metrical(u15::from(PPQN)),
        ));

        let params = &self.source.header.params;

        let key = *self.get_header_param("key", params.key.as_ref())?;
        let voices = self.get_header_param("voices", params.voices.as_ref())?;
        let time = self.get_header_param("time", params.time.as_ref())?;
        let tempo = self.get_header_param("tempo", params.tempo.as_ref())?;

        let tempo_track = self.create_tempo_track(tempo, time);

        smf.tracks.push(tempo_track);

        for (id, voice) in voices.iter().enumerate() {
            let track = self.process_voice(id, voice.value, key, time.bottom)?;
            smf.tracks.push(track);
        }

        Ok(smf)
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

    fn process_voice(
        &mut self,
        id: usize,
        voice: Voice,
        key: Key,
        quarter_unit: usize,
    ) -> Result<Track<'static>> {
        let voice_line = self.source.build_voice_line(voice);
        let params = PlaybackParams::new(key, quarter_unit);
        let evaluator = TimelineEvaluator::new(params);
        let task = VoiceTask::new(id, voice, evaluator);
        let track = task.process(&voice_line)?;

        Ok(track)
    }

    fn create_tempo_track(
        &self,
        initial_tempo: &Tempo,
        initial_time: &TimeSignature,
    ) -> Track<'static> {
        let mut task = TempoTask::new(initial_tempo, initial_time);

        for section in &self.source.ir.sections {
            let ticks = PPQN as u32 * section.items[0].views.len() as u32;

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

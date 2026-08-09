pub mod error;

mod task;

#[cfg(test)]
mod tests;

use midly::{Format, Header, Smf, Timing, num::u15};
use voxalfa_core::{
    data_types::Voice,
    output::{
        FinalOutput,
        evaluator::{PlaybackParams, TimelineEvaluator},
    },
};

use crate::{
    error::{Error, Result},
    task::{ConverterTask, TaskResult},
};

pub const PPQN: u16 = 480;
pub const MAX_PAUSE: u32 = (PPQN as u32) / 20;
pub const BASE_MIDI_KEY: i8 = 60; // middle C

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

        let header = &self.source.header;

        let key = self.extract_field("key", header.get_params(|p| &p.key))?;
        let time = self.extract_field("time", header.get_params(|p| &p.time))?;
        let tempo = self.extract_field("tempo", header.get_params(|p| &p.tempo))?;
        let voices = self.extract_field("voices", header.get_params(|p| &p.voices))?;

        let params = PlaybackParams::new(*key, *time, *tempo);
        let mut total_ticks = 0;

        for (id, voice) in voices.iter().enumerate() {
            let result = self.process_voice(id, voice.value, params.clone())?;

            if id == 0 {
                smf.tracks.push(result.meta_track);
                total_ticks = result.ticks;
            } else if total_ticks != result.ticks {
                return Err(Error::OutOfSync(id, result.ticks, total_ticks));
            }

            smf.tracks.push(result.voice_track);
        }

        Ok(smf)
    }

    fn process_voice(
        &mut self,
        id: usize,
        voice: Voice,
        params: PlaybackParams,
    ) -> Result<TaskResult> {
        let voice_line = self.source.build_voice_line(voice);
        let evaluator = TimelineEvaluator::new(params);
        let task = ConverterTask::new(id, voice_line.voice, evaluator);
        let track = task.process(&voice_line)?;

        Ok(track)
    }

    fn extract_field<T>(&self, name: &'static str, param: Option<&'a T>) -> Result<&'a T> {
        param.ok_or(Error::MissingHeaderField(name))
    }
}

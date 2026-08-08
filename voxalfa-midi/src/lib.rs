pub mod error;

mod task;

#[cfg(test)]
mod tests;

use midly::{Format, Header, Smf, Timing, num::u15};
use voxalfa_core::{
    ast::symbols::SymbolRef,
    data_types::Voice,
    output::{
        FinalOutput,
        evaluator::{PlaybackParams, TimelineEvaluator},
    },
};

use crate::{
    error::{ConvertError, Result},
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

        let init_params = &self.source.header.params;

        let key = *self.get_header_param("key", init_params.key.as_ref())?;
        let voices = self.get_header_param("voices", init_params.voices.as_ref())?;
        let time = self.get_header_param("time", init_params.time.as_ref())?;
        let tempo = self.get_header_param("tempo", init_params.tempo.as_ref())?;

        let params = PlaybackParams::new(key, *time, *tempo);
        let mut total_ticks = 0;

        for (id, voice) in voices.iter().enumerate() {
            let result = self.process_voice(id, voice.value, params.clone())?;

            if id == 0 {
                smf.tracks.push(result.meta_track);
                total_ticks = result.ticks;
            } else if total_ticks != result.ticks {
                return Err(ConvertError::OutOfSync(id, result.ticks, total_ticks));
            }

            smf.tracks.push(result.voice_track);
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
        params: PlaybackParams,
    ) -> Result<TaskResult> {
        let voice_line = self.source.build_voice_line(voice);
        let evaluator = TimelineEvaluator::new(params);
        let task = ConverterTask::new(id, voice_line.voice, evaluator);
        let track = task.process(&voice_line)?;

        Ok(track)
    }
}

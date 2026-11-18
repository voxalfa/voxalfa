pub mod error;

mod task;
mod types;

use quick_xml::se::to_string_with_root;
use voxalfa_core::{ast::symbols::SymbolRef, output::FinalOutput};

use crate::{
    error::{Error, Result},
    task::MeasureTask,
    types::{Part, ScorePart, ScorePartwise},
};

const HEADER: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="no"?>
<!DOCTYPE score-partwise PUBLIC
    "-//Recordare//DTD MusicXML 4.0 Partwise//EN"
    "http://www.musicxml.org/dtds/partwise.dtd">"#;

#[derive(Debug)]
pub struct Converter<'a> {
    source: &'a FinalOutput,
}

impl<'a> Converter<'a> {
    pub fn new(source: &'a FinalOutput) -> Self {
        Self { source }
    }

    pub fn convert(self) -> Result<String> {
        let params = &self.source.header.params;

        let key = *self.extract_field("key", params.key.as_ref())?;
        let time = *self.extract_field("time", params.time.as_ref())?;
        let voices = self.extract_field("voice", params.voices.as_ref())?;

        let mut score = ScorePartwise::default();

        for (voice_id, voice) in voices.iter().enumerate() {
            score.part_list.score_part.push(ScorePart {
                id: voice_id.to_string(),
                part_name: voice.value.to_string(),
            });

            let voice_line = self.source.build_voice_line(voice_id);
            let task = MeasureTask::new(voice.value, key, time);
            let measures = task.process(voice_line);

            score.part.push(Part {
                id: voice_id.to_string(),
                measure: measures,
            });
        }

        let serialized = to_string_with_root("score-partwise", &score)?;
        let xml = format!("{HEADER}\n{serialized}");

        Ok(xml)
    }

    fn extract_field<T>(
        &self,
        name: &'static str,
        param: Option<&'a SymbolRef<T>>,
    ) -> Result<&'a T> {
        param
            .map(|p| &p.value)
            .ok_or(Error::MissingHeaderField(name))
    }
}

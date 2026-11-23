pub mod error;

mod lyric;
mod task;
mod types;

use quick_xml::se::to_string_with_root;
use voxalfa_core::{
    ast::symbols::SymbolRef,
    output::{FinalOutput, lyrics::LyricsBuilder, metrics::DummyMeasurer},
};

use crate::{
    error::{Error, Result},
    lyric::LyricFormatter,
    task::MeasureTask,
    types::{Credit, Part, ScorePart, ScorePartwise},
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

        let lyrics_builder = LyricsBuilder::new(DummyMeasurer {});
        let (_, lyrics_map) = lyrics_builder.build_map::<LyricFormatter>(self.source, 0);

        let mut score = ScorePartwise {
            credit: self.build_credits(),
            ..Default::default()
        };

        for (voice_id, voice) in voices.iter().enumerate() {
            score.part_list.score_part.push(ScorePart {
                id: voice_id.to_string(),
                part_name: voice.value.to_string(),
            });

            let voice_line = self.source.build_voice_line(voice_id);
            let task = MeasureTask::new(voice.value, key, time, &lyrics_map);
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

    fn build_credits(&self) -> Vec<Credit> {
        let header = &self.source.header;
        let mut credits = Vec::new();

        if let Some(title) = header.get_metadata(|m| &m.title) {
            credits.push(Credit::new("title", title));
        }

        if let Some(composers) = header.get_metadata(|m| &m.composer) {
            credits.push(Credit::new("composer", join_metadata(composers)));
        }

        if let Some(authors) = header.get_metadata(|m| &m.author) {
            credits.push(Credit::new("lyricist", join_metadata(authors)));
        }

        credits
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

fn join_metadata(values: &[SymbolRef<String>]) -> String {
    values
        .iter()
        .map(|item| item.value.as_str())
        .collect::<Vec<_>>()
        .join(", ")
}

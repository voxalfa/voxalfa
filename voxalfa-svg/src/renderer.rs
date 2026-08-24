use crate::{
    error::{Error, Result},
    fonts::FontInterface,
    types::{Barline, LineSystem, TextElement},
    visitor::SvgVisitor,
};

use voxalfa_core::{
    ast::symbols::{ScopeId, SymbolRef},
    data_types::TimeSignature,
    ir::{SectionIr, solfa::NoteKind},
    output::{
        FinalOutput,
        lyrics::{LyricsBuilder, LyricsMap, RenderedLyric},
        voice::VoiceSet,
    },
};

pub struct Renderer<'a> {
    data: FinalOutput,
    font: FontInterface<'a>,
    time: TimeSignature,
    col_width: f32,
    lyrics_map: LyricsMap<f32>,
    text_elements: Vec<TextElement>,
    barlines: Vec<Barline>,
}

impl<'a> Renderer<'a> {
    pub fn new(data: FinalOutput) -> Result<Self> {
        let font = FontInterface::new()?;
        let max_factor = data.resolve_maximum_factor();
        let time = data.header.get_params(|p| &p.time);
        let builder = LyricsBuilder::new(&font);
        let (col_width, lyrics_map) = builder.build_map::<SvgVisitor>(&data, max_factor);

        if let Some(&time) = time {
            Ok(Self {
                data,
                font,
                time,
                col_width,
                lyrics_map,
                text_elements: Vec::new(),
                barlines: Vec::new(),
            })
        } else {
            Err(Error::MissingHeaderField("time"))
        }
    }

    fn build_systems(&mut self) {
        let systems = self.collect_systems();

        //
    }

    fn collect_systems(&self) -> Vec<LineSystem<'_>> {
        let mut result = Vec::new();
        let mut current_group = LineSystem::default();

        for section in &self.data.body.sections {
            let voices = section.voice_sets();

            if current_group.voices != voices {
                result.push(current_group.take());
            }

            current_group.internals.push(section);
        }

        result
    }
}

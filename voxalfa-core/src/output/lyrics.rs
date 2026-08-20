use std::collections::HashMap;

use crate::{
    ast::{
        lyrics::{LyricOperatorKind, LyricSpecialChar},
        symbols::SymbolTree,
    },
    data_types::Voice,
    ir::{
        BodyIr,
        lyrics::{LyricColumnIR, LyricStringIR},
    },
    output::{
        FinalOutput,
        evaluator::{PlaybackParams, TimelineEvaluator},
        metrics::StringMetric,
        voice::{NoteContext, VoiceLine},
    },
};

pub type LyricId = usize;
pub type LyricsMap<W> = HashMap<LyricId, RenderedLyric<W>>;

#[derive(Debug)]
pub enum LyricEvent<'a> {
    UnderlineStart,
    UnderlineEnd,
    GroupStart,
    GroupEnd,
    Placeholder,
    Operator(LyricOperatorKind),
    Text(&'a str),
    SpecialChar(LyricSpecialChar),
    Span(usize),
}

pub trait LyricVisitor: Default {
    fn get_operator(operator: LyricOperatorKind) -> Option<char>;
    fn handle_event(&mut self, event: LyricEvent);
    fn into_string(self) -> String;
}

#[derive(Debug)]
pub struct RenderedLyric<W> {
    pub content: String,
    pub width: W,
}

pub struct LyricsBuilder<M: StringMetric> {
    measurer: M,
    map: LyricsMap<M::Output>,
    max_width: M::Output,
}

impl<M: StringMetric> LyricsBuilder<M> {
    pub fn new(measurer: M) -> Self {
        Self {
            measurer,
            map: HashMap::new(),
            max_width: Default::default(),
        }
    }

    pub fn build_map<V: LyricVisitor>(
        mut self,
        output: &FinalOutput,
        max_factor: u8,
    ) -> (M::Output, LyricsMap<M::Output>) {
        let tree = &output.symbols;
        let sub_sections = output.body.sections.iter().flat_map(|s| &s.items);

        for section in sub_sections {
            let view_columns = section
                .views
                .iter()
                .flat_map(|v| v.durations.iter().map(|d| (*d, v.factor)))
                .collect::<Vec<_>>();

            for line in &section.lyrics {
                let mut col_index = 0;

                for column in &line.columns {
                    let rendered = self.build_lyric_column::<V>(column, tree);
                    let (duration, factor) = &view_columns[col_index];
                    let is_unit_col = column.span == 1 && *duration == 1 && *factor == max_factor;

                    // TODO: consider out of bounds edge cases?
                    if is_unit_col && rendered.width > self.max_width {
                        self.max_width = rendered.width;
                    }

                    self.map.insert(column.sid, rendered);
                    col_index += column.span;
                }
            }
        }

        (self.max_width, self.map)
    }

    fn build_lyric_column<V: LyricVisitor>(
        &self,
        column: &LyricColumnIR,
        tree: &SymbolTree,
    ) -> RenderedLyric<M::Output> {
        let mut visitor = V::default();

        if column.placeholder {
            visitor.handle_event(LyricEvent::Placeholder);
        } else {
            let is_group = column.chunks.len() > 1;

            if is_group {
                visitor.handle_event(LyricEvent::GroupStart);
            }

            for (chunk_id, chunk) in column.chunks.iter().enumerate() {
                for primitive in &chunk.primitives {
                    if primitive.underline.left {
                        visitor.handle_event(LyricEvent::UnderlineStart);
                    }

                    match primitive.string {
                        LyricStringIR::Reference(id) => {
                            visitor.handle_event(LyricEvent::Text(tree.get_lyric_chunk(id)))
                        }
                        LyricStringIR::Special(ch) => {
                            visitor.handle_event(LyricEvent::SpecialChar(ch));
                        }
                    };

                    if primitive.underline.right {
                        visitor.handle_event(LyricEvent::UnderlineEnd);
                    }
                }

                if let Some(operator) = column.operators.get(chunk_id) {
                    visitor.handle_event(LyricEvent::Operator(*operator));
                }
            }

            if is_group {
                visitor.handle_event(LyricEvent::GroupEnd);
            }
        }

        visitor.handle_event(LyricEvent::Span(column.span));

        let content = visitor.into_string();
        let width = self.measurer.measure_string(&content);

        RenderedLyric { content, width }
    }
}

// TODO: lyrics merging support for multiple voices
#[derive(Debug, PartialEq)]
pub struct LyricToken {
    pub lyric_id: LyricId,
    pub section_id: usize,
    pub sub_section_id: usize,
    pub operator: Option<LyricOperatorKind>,
}

#[derive(Debug)]
pub struct LyricsResolver {
    voice: Voice,
    lyrics_map: LyricsMap<()>,
}

impl LyricsResolver {
    pub fn new(voice: Voice, lyrics_map: LyricsMap<()>) -> Self {
        Self { voice, lyrics_map }
    }

    pub fn process<V: LyricVisitor>(self, data: &FinalOutput) -> Vec<String> {
        let verse_tokens = self.resolve_voice_tokens(self.voice, data);
        let mut results = Vec::new();

        for tokens in verse_tokens {
            let mut buffer = String::new();

            for token in tokens {
                buffer.push_str(&self.lyrics_map[&token.lyric_id].content);

                if let Some(ch) = token.operator.and_then(V::get_operator) {
                    buffer.push(ch);
                }
            }

            results.push(buffer);
        }

        results
    }

    fn resolve_voice_tokens(&self, voice: Voice, data: &FinalOutput) -> Vec<Vec<LyricToken>> {
        let mut result = Vec::new();
        let voice_line = &data.build_voice_line(voice);

        let verses = data
            .header
            .get_metadata(|m| &m.verses)
            .copied()
            .unwrap_or_default();

        for verse in 0..verses {
            let tokens = self.resolve_verse_tokens(verse, voice_line, &data.body);
            result.push(tokens);
        }

        result
    }

    fn resolve_verse_tokens(
        &self,
        verse: usize,
        voice_line: &VoiceLine,
        body: &BodyIr,
    ) -> Vec<LyricToken> {
        let mut tokens: Vec<LyricToken> = Vec::new();
        let mut evaluator = TimelineEvaluator::new(PlaybackParams::dummy());

        while let Some(context) = voice_line.notes.get(evaluator.index()) {
            evaluator.handle_events(&voice_line.timeline);

            if evaluator.done() {
                break;
            }

            if evaluator.jump() {
                if let Some(last) = tokens.last_mut() {
                    last.operator = Some(LyricOperatorKind::Newline);
                }

                continue;
            }

            if !evaluator.is_waiting()
                && let Some(token) = self
                    .resolve_lyric_token(verse, context, body)
                    .filter(|t| tokens.last() != Some(t))
            {
                tokens.push(token);
            }

            evaluator.step();

            if evaluator.index() >= voice_line.notes.len() {
                evaluator.handle_events(&voice_line.timeline);
                evaluator.jump();
            }
        }

        tokens
    }

    fn resolve_lyric_token(
        &self,
        verse: usize,
        context: &NoteContext,
        body: &BodyIr,
    ) -> Option<LyricToken> {
        let section = &body.sections[context.section_id];
        let sub_section = &section.items[context.sub_section_id];
        let line = sub_section.lyrics.get(verse)?;

        let mut current_id = 0;

        for (index, column) in line.columns.iter().enumerate() {
            if current_id == context.lyric_id {
                let operator = line.operators.get(index).map(|s| s.value);

                return Some(LyricToken {
                    lyric_id: column.sid,
                    section_id: context.section_id,
                    sub_section_id: context.sub_section_id,
                    operator,
                });
            }

            if current_id > context.lyric_id {
                break;
            }

            current_id += column.span;
        }

        None
    }
}

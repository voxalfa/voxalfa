use std::collections::HashMap;

use crate::{
    ast::{
        lyrics::{LyricOperatorKind, LyricSpecialChar},
        symbols::SymbolTree,
    },
    ir::lyrics::{LyricColumnIR, LyricStringIR},
    output::{
        FinalOutput,
        evaluator::{PlaybackParams, TimelineEvaluator},
        metrics::StringMetric,
        voice::VoiceLine,
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

#[derive(Debug)]
pub struct LyricsEvaluator {
    context: TimelineEvaluator,
    results: Vec<String>,
    _lyrics_map: LyricsMap<()>,
}

impl LyricsEvaluator {
    pub fn new(_lyrics_map: LyricsMap<()>) -> Self {
        Self {
            context: TimelineEvaluator::new(PlaybackParams::dummy()),
            results: Vec::new(),
            _lyrics_map,
        }
    }

    pub fn process(mut self, voice_line: &VoiceLine) -> Vec<String> {
        while let Some(_ctx) = voice_line.notes.get(self.context.index()) {
            self.context.handle_events(&voice_line.timeline);

            if self.context.done() {
                break;
            }

            if self.context.jump() {
                continue;
            }

            if !self.context.is_waiting() {
                // todo: get note's lyrics
            }

            self.context.step();

            if self.context.index() >= voice_line.notes.len() {
                self.context.handle_events(&voice_line.timeline);
                self.context.jump();
            }
        }

        self.results
    }
}

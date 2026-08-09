pub mod dynamics;
pub mod evaluator;
pub mod event;
pub mod lyrics;
pub mod tempo;
pub mod voice;

use tree_sitter::Tree;

use crate::{
    ast::{
        header::{Header, HeaderMetadata},
        params::InitialParams,
        symbols::{SymbolRef, SymbolTree},
    },
    data_types::Voice,
    diagnostics::types::Diagnostic,
    ir::{
        BodyIr, PulseView,
        lyrics::{LyricColumnIR, LyricLineIr, LyricPrimitive, LyricStringIR},
        solfa::{PulseColumn, PulseColumnKind},
    },
    output::{
        event::TimelineMap,
        voice::{NoteContext, VoiceLine},
    },
};

pub const MIN_COLUMN_WIDTH: usize = 4;

#[derive(Debug, Default)]
pub struct FinalOutput {
    pub tree: Option<Tree>,
    pub symbols: SymbolTree,
    pub header: Header,
    pub body: BodyIr,
    pub diagnostics: Vec<Diagnostic>,
    pub timelines: TimelineMap,
}

impl FinalOutput {
    pub fn with_tree(mut self, tree: Tree) -> Self {
        self.tree = Some(tree);
        self
    }

    pub fn has_error(&self) -> bool {
        self.diagnostics.iter().any(|d| d.is_error())
    }

    pub fn has_syntax_error(&self) -> bool {
        self.diagnostics.iter().any(|d| d.is_syntactic())
    }

    pub fn get_header_metadata<F, T>(&self, getter: F) -> Option<&T>
    where
        F: Fn(&HeaderMetadata) -> Option<&SymbolRef<T>>,
    {
        getter(&self.header.metadata).as_ref().map(|f| &f.value)
    }

    pub fn get_header_params<F, T>(&self, getter: F) -> Option<&T>
    where
        F: Fn(&InitialParams) -> Option<&SymbolRef<T>>,
    {
        getter(&self.header.params).as_ref().map(|f| &f.value)
    }

    pub fn resolve_column_width(&self) -> usize {
        let max_note_width = self.resolve_max_note_width();
        let max_lyrics_width = self.resolve_max_lyrics_width();

        max_lyrics_width.max(max_note_width).max(MIN_COLUMN_WIDTH)
    }

    pub fn resolve_column_factor(&self) -> usize {
        self.body
            .sections
            .iter()
            .flat_map(|s| &s.items)
            .flat_map(|s| &s.solfa)
            .flat_map(|s| s.pulses.iter().map(|p| p.factor).max())
            .max()
            .unwrap_or(1)
    }

    pub fn build_voice_line(&self, voice: Voice) -> VoiceLine<'_> {
        let mut notes = Vec::new();
        let mut timeline = Vec::new();
        let mut lyric_id = 0;
        let mut pulse_id = 0;

        let sub_items = self.body.sections.iter().flat_map(|section| &section.items);

        for sub in sub_items {
            let Some(solfa) = sub.solfa.iter().find(|s| s.voice == voice) else {
                continue;
            };

            if let Some(partial) = self.timelines.get(sub.sid) {
                timeline.extend(partial);
            }

            for (id, pulse) in solfa.pulses.iter().enumerate() {
                let view = &sub.views[id];

                for note in &pulse.columns {
                    notes.push(NoteContext {
                        note,
                        lyric_id,
                        pulse_id,
                        factor: pulse.factor,
                    });

                    if view.factor > 1 {
                        lyric_id += 1
                    }
                }

                if view.factor == 1 {
                    lyric_id += 1;
                }

                pulse_id += 1;
            }
        }

        VoiceLine::new(voice, notes, timeline)
    }

    // FIXME: use a dedicated lyric builder task for handling jumps
    pub fn build_lyrics(&self, voice: Voice, ulhs: &str, urhs: &str) -> Vec<String> {
        let mut result = Vec::new();

        let section_verses: Vec<&[LyricLineIr]> = self
            .body
            .sections
            .iter()
            .filter_map(|s| s.get_verses(&voice))
            .collect();

        let max_lines = section_verses.iter().map(|v| v.len()).max().unwrap_or(0);

        for line_idx in 0..max_lines {
            let verse = section_verses
                .iter()
                .filter_map(|verses| verses.get(line_idx))
                .map(|line| self.stringify_lyrics_line(line, ulhs, urhs))
                .collect::<Vec<String>>()
                .join("");

            result.push(verse);
        }

        result
    }

    fn stringify_lyrics_line(&self, line: &LyricLineIr, ulhs: &str, urhs: &str) -> String {
        let mut result = String::new();

        for (column_id, column) in line.columns.iter().enumerate() {
            if column.placeholder {
                continue;
            }

            let column_str = self.stringify_lyrics_column(column, ulhs, urhs);

            result.push_str(&column_str);

            if let Some(ch) = line.operators.get(column_id).and_then(|op| op.value.char()) {
                result.push(ch);
            }
        }

        result
    }

    pub fn stringify_lyrics_column(
        &self,
        column: &LyricColumnIR,
        ulhs: &str,
        urhs: &str,
    ) -> String {
        let mut result = String::new();

        for (chunk_id, chunk) in column.chunks.iter().enumerate() {
            for primitive in &chunk.primitives {
                if primitive.underline.left {
                    result.push_str(ulhs);
                }

                let part = match primitive.string {
                    LyricStringIR::Reference(id) => self.symbols.get_lyric_chunk(id),
                    LyricStringIR::Special(ch) => &ch.to_string(),
                };

                result.push_str(part);

                if primitive.underline.right {
                    result.push_str(urhs);
                }
            }

            if let Some(ch) = column.operators.get(chunk_id).and_then(|op| op.char()) {
                result.push(ch);
            }
        }

        result
    }

    fn resolve_lyric_column_width(&self, column: &LyricColumnIR) -> usize {
        let extra = if column.chunks.len() > 1 { 2 } else { 0 }; // add parenthesis

        column
            .chunks
            .iter()
            .map(|c| self.resolve_lyric_string_width(&c.primitives))
            .sum::<usize>()
            + column.operators.len()
            + extra
    }

    fn resolve_lyric_string_width(&self, strings: &[LyricPrimitive]) -> usize {
        strings
            .iter()
            .map(|s| self.resolve_primitive_width(s))
            .sum()
    }

    fn resolve_primitive_width(&self, s: &LyricPrimitive) -> usize {
        let base_width = match &s.string {
            LyricStringIR::Reference(id) => self.symbols.get_lyric_chunk(*id).chars().count(),
            LyricStringIR::Special(_) => 4,
        };

        base_width + (s.underline.left as usize) + (s.underline.right as usize)
    }

    fn resolve_max_lyrics_width(&self) -> usize {
        let max_factor = self.resolve_column_factor();

        self.body
            .sections
            .iter()
            .flat_map(|s| &s.items)
            .flat_map(|s| self.filter_lyric_columns(&s.lyrics, &s.views, max_factor))
            .map(|col| self.resolve_lyric_column_width(col))
            .max()
            .unwrap_or(1)
    }

    fn filter_lyric_columns<'a>(
        &self,
        lyrics: &'a [LyricLineIr],
        views: &[PulseView],
        max_factor: usize,
    ) -> Vec<&'a LyricColumnIR> {
        let view_columns = views
            .iter()
            .flat_map(|v| v.durations.iter().map(|d| (*d, v.factor)))
            .collect::<Vec<_>>();

        let mut result = Vec::new();

        for line in lyrics {
            let mut col_index = 0;

            for lyric_col in &line.columns {
                let (duration, factor) = &view_columns[col_index];

                // TODO: consider out of bounds edge cases
                if lyric_col.span == 1 && *duration == 1 && *factor == max_factor {
                    result.push(lyric_col);
                }

                col_index += lyric_col.span;
            }
        }

        result
    }

    fn resolve_max_note_width(&self) -> usize {
        self.body
            .sections
            .iter()
            .flat_map(|s| &s.items)
            .flat_map(|s| &s.solfa)
            .flat_map(|s| &s.pulses)
            .flat_map(|p| &p.columns)
            .map(|c| self.resolve_note_width(c))
            .max()
            .unwrap_or(1)
    }

    fn resolve_note_width(&self, column: &PulseColumn) -> usize {
        let base = match column.kind {
            PulseColumnKind::Note(note) => note.width(),
            PulseColumnKind::ProlongedNote => 1,
            PulseColumnKind::EmptyNote => 1,
        };

        let extra = column.underline.right ^ column.underline.left;

        base + extra as usize
    }
}

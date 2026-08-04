pub mod dynamics;
pub mod evaluator;
pub mod event;
pub mod render;
pub mod tempo;
pub mod voice;

use crate::{
    ast::{
        header::Header,
        symbols::{Delimiter, SymbolTree},
    },
    data_types::Voice,
    diagnostics::types::Diagnostic,
    ir::{
        BodyIR, PulseView,
        lyrics::{LyricColumnIR, LyricLineIR, LyricPrimitive, LyricStringIR},
        solfa::PulseColumnKind,
    },
    output::{
        event::TimelineMap,
        render::RenderType,
        voice::{NoteContext, VoiceLine},
    },
};

#[derive(Debug)]
pub struct FinalOutput {
    pub tree: SymbolTree,
    pub header: Header,
    pub ir: BodyIR,
    pub diagnostics: Vec<Diagnostic>,
    pub timelines: TimelineMap,
    pub delimiters: Vec<Delimiter>,
}

impl FinalOutput {
    pub fn has_error(&self) -> bool {
        self.diagnostics.iter().any(|d| d.is_error())
    }

    pub fn resolve_column_width(&self, render_type: RenderType) -> usize {
        let max_note_width = self.resolve_max_note_width(render_type);
        let max_lyrics_width = self.resolve_max_lyrics_width(render_type);

        max_lyrics_width.max(max_note_width).max(3)
    }

    pub fn resolve_column_factor(&self) -> usize {
        self.ir
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

        let sub_items = self.ir.sections.iter().flat_map(|section| &section.items);

        for sub in sub_items {
            let Some(solfa) = sub.solfa.iter().find(|s| s.voice == voice) else {
                continue;
            };

            if let Some(partial) = self.timelines.get(sub.sid) {
                timeline.extend(partial);
            }

            for pulse in &solfa.pulses {
                for note in &pulse.columns {
                    notes.push(NoteContext {
                        note,
                        factor: pulse.factor,
                    });
                }
            }
        }

        VoiceLine::new(voice, notes, timeline)
    }

    // FIXME: use a dedicated lyric builder task for handling jumps
    pub fn build_lyrics(&self, voice: Voice, ulhs: &str, urhs: &str) -> Vec<String> {
        let mut result = Vec::new();

        let section_verses: Vec<&[LyricLineIR]> = self
            .ir
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

    fn stringify_lyrics_line(
        &self,
        line: &LyricLineIR,
        underline_lhs: &str,
        underline_rhs: &str,
    ) -> String {
        let mut result = String::new();

        for (column_id, column) in line.columns.iter().enumerate() {
            for (chunk_id, chunk) in column.chunks.iter().enumerate() {
                for primitive in &chunk.primitives {
                    if primitive.underline.left {
                        result.push_str(underline_lhs);
                    }

                    let part = match primitive.string {
                        LyricStringIR::Reference(id) => self.tree.get_lyric_chunk(id),
                        LyricStringIR::Special(ch) => &ch.to_string(),
                    };

                    result.push_str(part);

                    if primitive.underline.right {
                        result.push_str(underline_rhs);
                    }
                }

                if let Some(ch) = column.operators.get(chunk_id).and_then(|op| op.char()) {
                    result.push(ch);
                }
            }

            if let Some(ch) = line.operators.get(column_id).and_then(|op| op.char()) {
                result.push(ch);
            }
        }

        result
    }

    fn resolve_lyric_column_width(&self, column: &LyricColumnIR, render_type: RenderType) -> usize {
        let extra = if column.chunks.len() > 1 { 2 } else { 0 }; // add parenthesis

        column
            .chunks
            .iter()
            .map(|c| self.resolve_lyric_string_width(&c.primitives, render_type))
            .sum::<usize>()
            + column.operators.len()
            + extra
    }

    fn resolve_lyric_string_width(
        &self,
        strings: &[LyricPrimitive],
        render_type: RenderType,
    ) -> usize {
        strings
            .iter()
            .map(|s| self.resolve_primitive_width(s, render_type))
            .sum()
    }

    fn resolve_primitive_width(&self, s: &LyricPrimitive, render_type: RenderType) -> usize {
        let base_width = match (&s.string, render_type) {
            (LyricStringIR::Reference(id), _) => self.tree.lyrics[*id].chars().count(),
            (LyricStringIR::Special(_), RenderType::Text) => 4,
            (LyricStringIR::Special(_), RenderType::Image) => 1,
        };

        if matches!(render_type, RenderType::Text) {
            base_width + (s.underline.left as usize) + (s.underline.right as usize)
        } else {
            base_width
        }
    }

    fn resolve_max_lyrics_width(&self, render_type: RenderType) -> usize {
        let max_factor = self.resolve_column_factor();

        self.ir
            .sections
            .iter()
            .flat_map(|s| &s.items)
            .flat_map(|s| self.filter_lyric_columns(&s.lyrics, &s.views, max_factor))
            .map(|col| self.resolve_lyric_column_width(col, render_type))
            .max()
            .unwrap_or(1)
    }

    fn filter_lyric_columns<'a>(
        &self,
        lyrics: &'a [LyricLineIR],
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

    fn resolve_max_note_width(&self, render_type: RenderType) -> usize {
        self.ir
            .sections
            .iter()
            .flat_map(|s| &s.items)
            .flat_map(|s| &s.solfa)
            .flat_map(|s| &s.pulses)
            .flat_map(|p| &p.columns)
            .map(|c| self.resolve_note_width(&c.kind, render_type))
            .max()
            .unwrap_or(1)
    }

    fn resolve_note_width(&self, column: &PulseColumnKind, render_type: RenderType) -> usize {
        match column {
            PulseColumnKind::Note(note) => note.width(render_type),
            PulseColumnKind::ProlongedNote => 1,
            PulseColumnKind::EmptyNote => 1,
        }
    }
}

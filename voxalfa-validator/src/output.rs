use crate::{
    ast::{
        header::Header,
        symbols::{Delimiter, SymbolTree},
    },
    data_types::Voice,
    diagnostics::types::Diagnostic,
    event::{Timeline, TimelineMap},
    ir::{
        BodyIR, PulseView,
        lyrics::{LyricColumnIR, LyricLineIR, LyricPrimitive, LyricStringIR},
        solfa::{PulseColumnKind, SolfaLineIR},
    },
    render::RenderType,
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

        max_lyrics_width.max(max_note_width)
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

    pub fn build_voice_sections<'a>(&'a self, voice: Voice) -> Vec<VoiceSection<'a>> {
        self.ir
            .sections
            .iter()
            .flat_map(|section| &section.items)
            .filter_map(|sub| {
                Some(VoiceSection {
                    voice,
                    timeline: self.timelines.get(sub.sid),
                    solfa: sub.solfa.iter().find(|s| s.voice == voice)?,
                })
            })
            .collect()
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
            PulseColumnKind::ProlongedNote(_) => 1,
            PulseColumnKind::EmptyNote => 1,
        }
    }
}

#[derive(Debug)]
pub struct VoiceSection<'a> {
    pub voice: Voice,
    pub timeline: Option<&'a Timeline>,
    pub solfa: &'a SolfaLineIR,
}

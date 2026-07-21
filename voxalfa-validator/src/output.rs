use crate::{
    ast::{
        document::Document,
        lyrics::{LyricChunkKind, LyricColumn, LyricString, LyricStringKind, LyricToken},
        symbols::SymbolTree,
    },
    diagnostic::Diagnostic,
    ir::DocumentIR,
    render::RenderType,
};

#[derive(Debug)]
pub struct ValidatorOutput {
    pub tree: SymbolTree,
    pub document: Document,
    pub ir: DocumentIR,
    pub diagnostics: Vec<Diagnostic>,
}

impl ValidatorOutput {
    pub fn resolve_column_width(&self, render_type: RenderType) -> usize {
        self.document
            .body
            .sections
            .iter()
            .flat_map(|s| &s.sub_sections)
            .flat_map(|s| &s.lyrics)
            .flat_map(|l| &l.tokens)
            .filter_map(|t| match t {
                LyricToken::Column(col) => Some(col),
                LyricToken::Operator(_) => None,
            })
            .map(|col| self.resolve_lyric_column_width(col, render_type))
            .max()
            .unwrap_or(1)
            .max(4) // FIXME: check all notes length
    }

    pub fn resolve_column_factor(&self) -> usize {
        self.ir
            .sections
            .iter()
            .flat_map(|s| &s.sub_sections)
            .flat_map(|s| {
                s.solfa
                    .iter()
                    .flat_map(|s| s.pulses.iter().map(|p| p.factor).max())
            })
            .max()
            .unwrap_or(1)
    }

    fn resolve_lyric_column_width(&self, column: &LyricColumn, render_type: RenderType) -> usize {
        let extra = if column.chunks.len() > 1 { 2 } else { 0 }; // add parenthesis

        column
            .chunks
            .iter()
            .map(|c| match (&c.value, render_type) {
                (LyricChunkKind::Space, _) => 1,
                (LyricChunkKind::Newline, RenderType::Text) => 1,
                (LyricChunkKind::String(strings), _) => {
                    self.resolve_lyric_string_width(strings, render_type)
                }
                _ => 0,
            })
            .sum::<usize>()
            + extra
    }

    fn resolve_lyric_string_width(
        &self,
        strings: &[LyricString],
        render_type: RenderType,
    ) -> usize {
        strings
            .iter()
            .map(|s| match (&s.value, render_type) {
                (LyricStringKind::UnderlineMarker, RenderType::Text) => 1,
                (LyricStringKind::SpecialChar(_), RenderType::Text) => 4,
                (LyricStringKind::Reference(id), _) => self.tree.lyrics[*id].chars().count(),
                _ => 0,
            })
            .sum()
    }
}

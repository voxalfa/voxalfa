use crate::{
    ast::lyrics::{LyricChunkKind, LyricColumn, LyricString, LyricStringKind, LyricToken},
    validator::ValidatorOutput,
};

impl ValidatorOutput {
    pub fn resolve_column_width(&self, text_size: bool) -> usize {
        self.document
            .body
            .sections
            .iter()
            .flat_map(|s| &s.lyrics)
            .flat_map(|l| &l.tokens)
            .filter_map(|t| match t {
                LyricToken::Column(col) => Some(col),
                LyricToken::Operator(_) => None,
            })
            .map(|col| self.resolve_lyric_column_width(col, text_size))
            .max()
            .unwrap_or(1)
            .max(4)
    }

    fn resolve_lyric_column_width(&self, column: &LyricColumn, text_size: bool) -> usize {
        column
            .chunks
            .iter()
            .map(|c| match &c.value {
                LyricChunkKind::Space => 1,
                LyricChunkKind::Newline if text_size => 1,
                LyricChunkKind::String(strings) => {
                    self.resolve_lyric_string_width(strings, text_size)
                }
                _ => 0,
            })
            .sum()
    }

    fn resolve_lyric_string_width(&self, strings: &[LyricString], text_size: bool) -> usize {
        strings
            .iter()
            .map(|s| match s.value {
                LyricStringKind::UnderlineMarker if text_size => 1,
                LyricStringKind::SpecialChar(_) if text_size => 4,
                LyricStringKind::Reference(id) => self.tree.lyrics[id].chars().count(),
                _ => 0,
            })
            .sum()
    }
}

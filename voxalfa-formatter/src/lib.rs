mod primitives;
mod types;

use std::io::{self, Write};

use voxalfa_validator::{
    ast::{
        body::SectionMetadata,
        header::HeaderMetadata,
        lyrics::LyricOperatorKind,
        params::{InitialParams, SectionParams, SubSectionParams},
        symbols::{Comment, SymbolRef},
    },
    ir::{
        PulseView,
        lyrics::{LyricColumnIR, LyricLineIR, LyricStringIR},
        solfa::{PulseIR, SolfaLineIR},
    },
    output::FinalOutput,
    render::RenderType,
    ts_utils::range::RangeUtil,
};

use crate::{
    primitives::Formattable,
    types::{Assignment, LineRank, PartialLine},
};

#[derive(Debug)]
pub struct Formatter<'a> {
    col_width: usize,
    col_factor: usize,
    source: &'a FinalOutput,
    partials: Vec<PartialLine>,
    mergable_lines: Vec<usize>,
    current_scope: usize,
    scope_bounds: Vec<usize>,
}

impl<'a> Formatter<'a> {
    pub fn new(source: &'a FinalOutput) -> Self {
        Self {
            col_width: source.resolve_column_width(RenderType::Text) + 1,
            col_factor: source.resolve_column_factor(),
            partials: Vec::new(),
            mergable_lines: Vec::new(),
            scope_bounds: Vec::new(),
            current_scope: 0,
            source,
        }
    }

    pub fn format<W: Write>(mut self, writer: &mut W) -> Result<(), io::Error> {
        let header = &self.source.header;

        self.process_metadata(&header.metadata);
        self.proces_initial_params(&header.params);
        self.push_delimiter();
        self.process_body();
        self.process_comments();

        self.finalize(writer)
    }

    fn finalize<W: Write>(mut self, writer: &mut W) -> Result<(), io::Error> {
        self.partials.sort();

        let mut lines = self.partials.into_iter().peekable();

        while let Some(line) = lines.next() {
            writer.write_all(line.content.as_bytes())?;

            if let Some(next) = lines.peek() {
                if next.rank != line.rank {
                    writer.write_all(b"\n\n")?;
                } else {
                    match next.line_id.saturating_sub(line.line_id) {
                        0 => {} // Same line
                        1 => writer.write_all(b"\n")?,
                        _ => writer.write_all(b"\n\n")?,
                    }
                }
            }
        }

        writer.write_all(b"\n")?;

        Ok(())
    }

    fn process_body(&mut self) {
        for (section_id, section) in self.source.ir.sections.iter().enumerate() {
            self.process_section_metadata(&section.metadata);
            self.proces_section_params(&section.params);

            for sub_section in &section.items {
                self.process_local_params(&sub_section.params);

                for solfa in &sub_section.solfa {
                    self.process_solfa(solfa);
                }

                for (lyrics_id, lyrics) in sub_section.lyrics.iter().enumerate() {
                    let verse = lyrics_id + 1;
                    let is_last_section = section_id == self.source.ir.sections.len() - 1;

                    self.process_lyrics(&sub_section.views, lyrics, verse, is_last_section);
                }

                self.push_delimiter();
            }
        }
    }

    fn process_metadata(&mut self, meta: &HeaderMetadata) {
        self.add_assignment(Assignment::Metadata, meta.title.as_ref());
        self.add_assignment(Assignment::Metadata, meta.author.as_ref());
        self.add_assignment(Assignment::Metadata, meta.composer.as_ref());
        self.add_assignment(Assignment::Metadata, meta.verses.as_ref());
        self.add_assignment(Assignment::Metadata, meta.meter.as_ref());
        self.add_assignment(Assignment::Metadata, meta.description.as_ref());
        self.add_assignment(Assignment::Metadata, meta.release.as_ref());
        self.add_assignment(Assignment::Metadata, meta.language.as_ref());
        self.add_assignment(Assignment::Metadata, meta.tags.as_ref());
    }

    fn process_section_metadata(&mut self, params: &SectionMetadata) {
        self.add_assignment(Assignment::Metadata, params.name.as_ref());
        self.add_assignment(Assignment::Metadata, params.ending.as_ref());
    }

    fn proces_section_params(&mut self, params: &SectionParams) {
        self.add_assignment(Assignment::Params, params.key.as_ref());
        self.add_assignment(Assignment::Params, params.time.as_ref());
        self.add_assignment(Assignment::Params, params.tempo.as_ref());
        self.add_assignment(Assignment::Params, params.navigation.as_ref());
    }

    fn proces_initial_params(&mut self, params: &InitialParams) {
        self.add_assignment(Assignment::Params, params.key.as_ref());
        self.add_assignment(Assignment::Params, params.time.as_ref());
        self.add_assignment(Assignment::Params, params.tempo.as_ref());
        self.add_assignment(Assignment::Params, params.voices.as_ref());
    }

    fn process_local_params(&mut self, params: &SubSectionParams) {
        self.add_assignment(Assignment::Params, params.dynamics.as_ref());
    }

    fn process_solfa(&mut self, solfa: &SolfaLineIR) {
        let scope = self.source.tree.get_scope(solfa.sid);
        let line_id = scope.range.start_point.row;
        let pulse_width = self.col_width * self.col_factor;
        let mut buffer = format!("[{:?}] ", solfa.voice);

        for pulse in &solfa.pulses {
            let pulse_str = self.format_pulse(pulse);
            let stretched = format!("{pulse_str:<pulse_width$}");
            buffer.push_str(&stretched);
        }

        buffer.push_str("||");

        self.push_line(LineRank::Solfa, line_id, buffer);
    }

    fn format_pulse(&mut self, pulse: &PulseIR) -> String {
        let mut buffer = String::new();
        let mut clock = 0;
        let accent = pulse.accent.to_string();

        for (step, column) in pulse.columns.iter().enumerate() {
            let lead = match (step, clock, pulse.factor) {
                (0, _, _) => &accent,
                (1, 1, 2) | (_, 2, 4) => ".",
                (1, 3, 4) => ".,",
                (_, 1, 4) | (_, 3, 4) => ",",
                _ if step == clock => ",", // n-uplets
                _ => "",
            };

            if lead.len() > 1 {
                buffer.pop();
            }

            let prefix_str = if column.underline.left { "`" } else { "" };
            let suffix_str = if column.underline.right { "`" } else { "" };
            let column_str = column.kind.to_string();

            let note = format!("{lead}{prefix_str}{column_str}{suffix_str}");
            let total_width = (self.col_width * column.duration * self.col_factor) / pulse.factor;
            let stretched = format!("{note:<total_width$}");

            buffer.push_str(&stretched);

            clock += column.duration;
        }

        buffer
    }

    fn process_lyrics(
        &mut self,
        views: &[PulseView],
        line: &LyricLineIR,
        verse: usize,
        is_last_section: bool,
    ) {
        let mut buffer = format!("[{verse}] ");
        let mut view_id = 0;
        let mut view_offset = 0;

        let scope = self.source.tree.get_scope(line.sid);
        let line_id = scope.range.start_point.row;
        let last_lyric_id = line.columns.len() - 1;
        let rank = LineRank::Lyrics;

        for (lyric_id, lyric_col) in line.columns.iter().enumerate() {
            let lyric_str = self.resolve_lyric_column(lyric_col);
            let char_count = lyric_str.chars().count();
            let operator = line.operators.get(lyric_id);

            let mut span_value = lyric_col.span;
            let mut width = 0;

            while span_value != 0 {
                let view = &views[view_id];
                let widths = view.resolve_widths(self.col_width, self.col_factor);

                width += widths[view_offset];
                view_offset += 1;
                span_value -= 1;

                if view_offset >= widths.len() {
                    width += self.col_width * self.col_factor - widths.iter().sum::<usize>();
                    view_id += 1;
                    view_offset = 0;
                }
            }

            let filler = match operator {
                Some(LyricOperatorKind::Concat) => "_",
                Some(LyricOperatorKind::Newline) => "\\",
                _ => "",
            };

            let padding = if is_last_section && lyric_id == last_lyric_id {
                0
            } else {
                width.saturating_sub(char_count)
            };

            let padded_str = format!("{lyric_str}{filler:<padding$}");

            buffer.push_str(&padded_str);

            if lyric_id == last_lyric_id && line.anchor {
                buffer.push_str("@@");
            }
        }

        self.push_line(rank, line_id, buffer);
    }

    fn resolve_lyric_column(&self, column: &LyricColumnIR) -> String {
        let mut buffer = String::new();

        if column.placeholder {
            buffer.push('~');
        }

        if column.chunks.len() > 1 {
            buffer.push('(');
        }

        for (id, chunk) in column.chunks.iter().enumerate() {
            for primitve in &chunk.primitives {
                if primitve.underline.left {
                    buffer.push('`');
                }

                let lyric_str = match primitve.string {
                    LyricStringIR::Reference(id) => self.source.tree.get_lyric_chunk(id),
                    LyricStringIR::Special(special) => special.identifer(),
                };

                buffer.push_str(lyric_str);

                if primitve.underline.right {
                    buffer.push('`');
                }
            }

            if let Some(operator) = column.operators.get(id) {
                let operator_char = match operator {
                    LyricOperatorKind::Space => ' ',
                    LyricOperatorKind::Newline => '\\',
                    LyricOperatorKind::Concat => unreachable!("concat should not be compound"),
                };

                buffer.push(operator_char);
            }
        }

        if column.chunks.len() > 1 {
            buffer.push(')');
        }

        if column.span > 1 {
            buffer.push_str(&format!("@{}", column.span));
        }

        buffer
    }

    fn process_comments(&mut self) {
        for comment in &self.source.tree.comments {
            let (line_id, scope, rank) = self.resolve_comment_position(comment);
            let trimmed = comment.value.trim_end();

            let content = match self.mergable_lines.contains(&line_id) {
                true => format!(" {trimmed}"),
                false => trimmed.to_string(),
            };

            self.partials.push(PartialLine {
                index: self.partials.len(),
                scope,
                rank,
                line_id,
                content,
            });
        }
    }

    fn resolve_comment_position(&self, comment: &Comment) -> (usize, usize, LineRank) {
        let line_id = self.source.tree.get_symbol_range(comment.sid).line();

        if comment.value.starts_with(";;") {
            let scope = self.scope_bounds.partition_point(|&line| line <= line_id);

            return (line_id, scope, LineRank::Directive);
        }

        let nearest = self.partials.iter().min_by_key(|p| {
            if p.line_id == line_id {
                (0, 0) // Priority 0: exact inline match
            } else if p.line_id > line_id {
                (1, p.line_id - line_id) // priority 1: closest line after comment
            } else {
                (2, line_id - p.line_id) // priority 2: closest line before comment (EOF fallback)
            }
        });

        nearest
            .map(|p| (line_id, p.scope, p.rank))
            .unwrap_or_default()
    }

    fn add_assignment<F: Formattable>(
        &mut self,
        kind: Assignment,
        symbol_ref: Option<&SymbolRef<F>>,
    ) {
        let Some(symbol) = symbol_ref else { return };

        let value_symbol = self.source.tree.get_symbol(symbol.sid);
        let scope = self.source.tree.get_scope(value_symbol.scope);
        let key_symbol = self.source.tree.get_symbol(scope.symbols[0]);
        let line_id = scope.range.start_point.row;

        let key_str = key_symbol.kind.as_key_unchecked();
        let value_str = symbol.value.format(false);
        let assignment_str = format!("{key_str}={value_str}");

        let prefix = kind.prefix();
        let rank = kind.rank();

        let content = match self.mergable_lines.contains(&line_id) {
            true => format!(" | {assignment_str}"),
            false => format!("[{prefix}] {assignment_str}"),
        };

        self.push_line(rank, line_id, content);
    }

    fn push_line(&mut self, rank: LineRank, line_id: usize, content: String) {
        self.partials.push(PartialLine {
            scope: self.current_scope,
            index: self.partials.len(),
            rank,
            line_id,
            content,
        });

        self.mergable_lines.push(line_id);
    }

    fn push_delimiter(&mut self) {
        if let Some(delimiter) = self.source.delimiters.get(self.current_scope) {
            self.partials.push(PartialLine {
                scope: self.current_scope,
                rank: LineRank::Delimiter,
                line_id: delimiter.line,
                content: delimiter.kind.to_string(),
                index: self.partials.len(),
            });

            self.scope_bounds.push(delimiter.line);
            self.current_scope += 1;
        }
    }
}

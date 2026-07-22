mod primitives;

use std::{
    collections::BTreeMap,
    io::{self, Write},
};

use voxalfa_validator::{
    ast::{
        dynamics::Dynamics, header::HeaderMetadata, lyrics::LyricOperatorKind,
        params::CompositionParams, symbols::SymbolRef,
    },
    ir::{
        PulseView,
        lyrics::{LyricColumnIR, LyricLineIR, LyricStringIR},
        solfa::{PulseIR, SolfaLineIR},
    },
    output::ValidatorOutput,
    render::RenderType,
};

use crate::primitives::Formattable;

#[derive(Debug)]
pub struct Formatter<'a> {
    col_width: usize,
    col_factor: usize,
    source: &'a ValidatorOutput,
    lines: BTreeMap<usize, String>,
    separators: BTreeMap<usize, String>,
}

impl<'a> Formatter<'a> {
    pub fn new(source: &'a ValidatorOutput) -> Self {
        Self {
            col_width: source.resolve_column_width(RenderType::Text) + 1,
            col_factor: source.resolve_column_factor(),
            lines: BTreeMap::new(),
            separators: BTreeMap::new(),
            source,
        }
    }

    pub fn format<W: Write>(mut self, writer: &mut W) -> Result<(), io::Error> {
        self.process_header();
        self.process_body();
        self.process_comments();

        self.finalize(writer)
    }

    fn finalize<W: Write>(self, writer: &mut W) -> Result<(), io::Error> {
        let mut lines = self.lines.into_iter().peekable();

        while let Some((line_id, line)) = lines.next() {
            writer.write_all(line.as_bytes())?;

            if let Some(separator) = self.separators.get(&line_id) {
                writer.write_all(separator.as_bytes())?;
            } else {
                if let Some((next_line_id, _)) = lines.peek()
                    && next_line_id - line_id > 1
                {
                    writer.write_all(b"\n")?;
                }

                if lines.peek().is_some() {
                    writer.write_all(b"\n")?;
                }
            }
        }

        Ok(())
    }

    fn process_header(&mut self) {
        self.process_metadata(&self.source.header.metadata);
        self.process_params(&self.source.header.params);

        self.append_separators("---");
    }

    fn process_body(&mut self) {
        for (section_id, section) in self.source.ir.sections.iter().enumerate() {
            self.append_separators("\n\n");
            self.process_params(&section.params);

            for (sub_id, sub) in section.items.iter().enumerate() {
                let sub_data = &section.items[sub_id];

                if sub_id > 0 {
                    self.append_separators("\n\n");
                }

                self.process_dynamics(&sub_data.dynamics);

                for solfa in &sub.solfa {
                    self.process_solfa(solfa);
                }

                for (lyrics_id, lyrics) in sub.lyrics.iter().enumerate() {
                    let verse = lyrics_id + 1;
                    let is_last_section = section_id == self.source.ir.sections.len() - 1;

                    self.process_lyrics(&sub.views, lyrics, verse, is_last_section);
                }

                if sub_id != section.items.len() - 1 {
                    self.append_separators("\n\n++");
                }
            }

            if section_id != self.source.ir.sections.len() - 1 {
                self.append_separators("\n\n--");
            }
        }
    }

    fn process_metadata(&mut self, meta: &HeaderMetadata) {
        let checkpoint = self.last_line();

        self.append_assignement("#", meta.title.as_ref());
        self.append_assignement("#", meta.author.as_ref());
        self.append_assignement("#", meta.composer.as_ref());
        self.append_assignement("#", meta.voices.as_ref());
        self.append_assignement("#", meta.verses.as_ref());
        self.append_assignement("#", meta.description.as_ref());
        self.append_assignement("#", meta.release.as_ref());
        self.append_assignement("#", meta.language.as_ref());
        self.append_assignement("#", meta.tags.as_ref());

        if checkpoint != self.last_line() {
            self.append_separators("\n\n");
        }
    }

    fn process_params(&mut self, params: &CompositionParams) {
        let checkpoint = self.last_line();

        self.append_assignement("$", params.key.as_ref());
        self.append_assignement("$", params.time.as_ref());
        self.append_assignement("$", params.bpm.as_ref());

        if checkpoint != self.last_line() {
            self.append_separators("\n\n");
        }
    }

    fn process_dynamics(&mut self, dynamics: &Dynamics) {
        for dynamic in &dynamics.value {
            self.append_assignement("^", Some(dynamic));
        }

        if !dynamics.value.is_empty() {
            self.append_separators("\n\n");
        }
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

        self.lines.insert(line_id, buffer);
    }

    fn format_pulse(&mut self, pulse: &PulseIR) -> String {
        let mut buffer = String::new();
        let mut clock = 0;

        for (step, column) in pulse.columns.iter().enumerate() {
            let lead = match (step, clock, pulse.factor) {
                (0, _, _) => &pulse.accent.to_string(),
                (1, 1, 2) | (_, 2, 4) => ".",
                (1, 3, 4) => ".,",
                (_, 1, 4) | (_, 3, 4) => ",",
                _ if step == clock => ",", // n-uplets
                _ => "",
            };

            let prefix_str = if column.underline.left { "`" } else { "" };
            let suffix_str = if column.underline.right { "`" } else { "" };
            let column_str = column.kind.to_string();

            let note = format!("{lead}{prefix_str}{column_str}");
            let total_width = (self.col_width * column.duration * self.col_factor) / pulse.factor;
            let padding_width = total_width.saturating_sub(suffix_str.len());

            let stretched = format!("{note:<padding_width$}{suffix_str}");

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

        for (lyric_id, lyric_col) in line.columns.iter().enumerate() {
            let lyric_str = self.resolve_lyric_column(lyric_col);
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
                    // width += self.col_width * self.col_factor - widths.iter().sum::<usize>();
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
                width.saturating_sub(lyric_str.chars().count())
            };

            let padded_str = format!("{lyric_str}{filler:<padding$}");

            buffer.push_str(&padded_str);

            if lyric_id == last_lyric_id && line.anchor {
                buffer.push_str("..");
            }
        }

        self.lines.insert(line_id, buffer);
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
            let symbol = self.source.tree.get_symbol(comment.sid);
            let line_id = symbol.range.start_point.row;

            if let Some(line) = self.lines.get_mut(&line_id) {
                line.push(' ');
                line.push_str(&comment.value);
            } else {
                self.lines.insert(line_id, comment.value.clone());
            }
        }
    }

    fn append_separators(&mut self, separator: &str) {
        let key = self.last_line();

        if let Some(value) = self.separators.get_mut(&key) {
            *value = value.to_owned() + separator;
        } else {
            self.separators.insert(key, separator.to_string());
        }
    }

    fn append_assignement<F: Formattable>(
        &mut self,
        prefix: &str,
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

        if let Some(line) = self.lines.get_mut(&line_id) {
            line.push_str(&format!(" | {assignment_str}"));
        } else {
            self.lines
                .insert(line_id, format!("[{prefix}] {assignment_str}"));
        }
    }

    fn last_line(&self) -> usize {
        self.lines
            .last_key_value()
            .map(|(&k, _)| k)
            .unwrap_or_default()
    }
}

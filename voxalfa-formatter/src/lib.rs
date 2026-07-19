mod literal;

use std::collections::BTreeMap;

use voxalfa_validator::{
    ast::{
        lyrics::LyricOperatorKind,
        symbols::{SymbolRef, SymbolTree},
    },
    diagnostic::Diagnostic,
    ir::{
        PulseView,
        lyrics::{LyricColumnIR, LyricLineIR, LyricStringIR},
        solfa::{PulseIR, SolfaLineIR},
    },
    output::ValidatorOutput,
    render::RenderType,
    ts_utils::context::TSContext,
    validator::DocumentValidator,
};

use crate::literal::Formattable;

#[derive(Debug, Default)]
pub struct Formatter {
    col_width: usize,
    col_factor: usize,
    lines: BTreeMap<usize, String>,
    separators: BTreeMap<usize, &'static str>,
}

impl Formatter {
    pub fn format(
        mut self,
        source: &str,
        ts_context: &mut TSContext,
    ) -> Result<String, Vec<Diagnostic>> {
        let validator = DocumentValidator::new(source);
        let output = validator.validate(ts_context);

        if output.diagnostics.iter().any(|d| d.is_error()) {
            return Err(output.diagnostics);
        }

        self.col_width = output.resolve_column_width(RenderType::Text) + 1;
        self.col_factor = output.resolve_column_factor();

        self.process_header(&output);
        self.process_body(&output);
        self.process_comments(&output.tree);

        Ok(self.finalize())
    }

    fn finalize(self) -> String {
        let mut buffer = String::new();
        let mut lines = self.lines.into_iter().peekable();

        while let Some((line_idx, line)) = lines.next() {
            buffer.push_str(&line);

            if let Some(separator) = self.separators.get(&line_idx) {
                buffer.push_str(separator);
            } else {
                if let Some((next_line_idx, _)) = lines.peek()
                    && next_line_idx - line_idx > 1
                {
                    buffer.push('\n');
                }

                if lines.peek().is_some() {
                    buffer.push('\n');
                }
            }
        }

        buffer
    }

    fn process_header(&mut self, output: &ValidatorOutput) {
        let meta = &output.document.header.metadata;
        let params = &output.document.header.params;

        self.append_assignement("#", &output.tree, meta.title.as_ref());
        self.append_assignement("#", &output.tree, meta.author.as_ref());
        self.append_assignement("#", &output.tree, meta.composer.as_ref());
        self.append_assignement("#", &output.tree, meta.release.as_ref());
        self.append_assignement("#", &output.tree, meta.description.as_ref());

        self.append_separators("\n\n");

        self.append_assignement("$", &output.tree, params.key.as_ref());
        self.append_assignement("$", &output.tree, params.time.as_ref());
        self.append_assignement("$", &output.tree, params.bpm.as_ref());
        self.append_assignement("$", &output.tree, params.voices.as_ref());

        self.append_separators("\n\n---\n\n");
    }

    fn process_body(&mut self, output: &ValidatorOutput) {
        for (section_idx, section) in output.ir.sections.iter().enumerate() {
            for (group_idx, group) in section.groups.iter().enumerate() {
                for solfa_idx in &group.solfa {
                    let solfa = &section.solfa[*solfa_idx];
                    self.process_solfa(&output.tree, solfa);
                }

                for (verse, lyrics_idx) in group.lyrics.iter().enumerate() {
                    let lyrics = &section.lyrics[*lyrics_idx];
                    self.process_lyrics(&output.tree, &group.views, lyrics, verse + 1);
                }

                if group_idx != section.groups.len() - 1 {
                    self.append_separators("\n\n");
                }
            }

            if section_idx != output.ir.sections.len() - 1 {
                self.append_separators("\n\n--\n\n");
            }
        }
    }

    fn process_solfa(&mut self, tree: &SymbolTree, solfa: &SolfaLineIR) {
        let scope = tree.get_scope(solfa.sid);
        let line_idx = scope.range.start_point.row;
        let pulse_width = self.col_width * self.col_factor;
        let mut buffer = format!("[{}] ", solfa.voice.format(true));

        for pulse in &solfa.pulses {
            let pulse_str = self.format_pulse(pulse);
            let stretched = format!("{pulse_str:<pulse_width$}");
            buffer.push_str(&stretched);
        }

        buffer.push_str("||");

        self.lines.insert(line_idx, buffer);
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
        tree: &SymbolTree,
        views: &[PulseView],
        lyrics: &LyricLineIR,
        verse: usize,
    ) {
        let mut buffer = format!("[{verse}] ");
        let mut view_idx = 0;
        let mut view_offset = 0;

        let scope = tree.get_scope(lyrics.sid);
        let line_idx = scope.range.start_point.row;
        let last_lyric_idx = lyrics.columns.len() - 1;

        for (lyric_idx, lyric_col) in lyrics.columns.iter().enumerate() {
            let lyric_str = self.resolve_lyric_column(tree, lyric_col);
            let operator = lyrics.operators.get(lyric_idx);

            let mut span_value = lyric_col.span;
            let mut width = 0;

            while span_value != 0 {
                let view = &views[view_idx];
                let widths = view.resolve_widths(self.col_width, self.col_factor);

                width += widths[view_offset];
                view_offset += 1;
                span_value -= 1;

                if view_offset >= widths.len() {
                    // width += self.col_width * self.col_factor - widths.iter().sum::<usize>();
                    view_idx += 1;
                    view_offset = 0;
                }
            }

            let filler = match operator {
                Some(LyricOperatorKind::Concat) => '_',
                Some(LyricOperatorKind::Newline) => '\\',
                _ => ' ',
            };

            let padding = width.saturating_sub(lyric_str.chars().count());
            let padded_str = format!(
                "{}{}",
                lyric_str,
                std::iter::repeat_n(filler, padding).collect::<String>()
            );

            buffer.push_str(&padded_str);

            if lyric_idx == last_lyric_idx && operator.is_some() {
                buffer.push_str("...");
            }
        }

        self.lines.insert(line_idx, buffer);
    }

    fn resolve_lyric_column(&self, tree: &SymbolTree, column: &LyricColumnIR) -> String {
        let mut buffer = String::new();

        if column.chunks.len() > 1 {
            buffer.push('(');
        }

        for (idx, chunk) in column.chunks.iter().enumerate() {
            if chunk.primitives.is_empty() {
                return '~'.to_string(); // placeholder
            }

            for primitve in &chunk.primitives {
                if primitve.underline.left {
                    buffer.push('`');
                }

                let lyric_str = match primitve.string {
                    LyricStringIR::Reference(id) => tree.get_lyric_chunk(id),
                    LyricStringIR::Special(special) => special.identifer(),
                };

                buffer.push_str(lyric_str);

                if primitve.underline.right {
                    buffer.push('`');
                }
            }

            if let Some(operator) = column.operators.get(idx) {
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

    fn process_comments(&mut self, tree: &SymbolTree) {
        for comment in &tree.comments {
            let symbol = tree.get_symbol(comment.sid);
            let line_idx = symbol.range.start_point.row;

            if let Some(line) = self.lines.get_mut(&line_idx) {
                line.push(' ');
                line.push_str(&comment.value);
            } else {
                self.lines.insert(line_idx, comment.value.clone());
            }
        }
    }

    fn append_separators(&mut self, separator: &'static str) {
        let last_line = self.lines.last_key_value().map(|(k, _)| k);

        if let Some(line) = last_line {
            self.separators.insert(*line, separator);
        }
    }

    fn append_assignement<F: Formattable>(
        &mut self,
        prefix: &str,
        tree: &SymbolTree,
        symbol_ref: Option<&SymbolRef<F>>,
    ) {
        let Some(symbol) = symbol_ref else { return };

        let value_symbol = tree.get_symbol(symbol.sid);
        let scope = tree.get_scope(value_symbol.scope);
        let key_symbol = tree.get_symbol(scope.symbols[0]);
        let line_idx = scope.range.start_point.row;

        let key_str = key_symbol.kind.as_key_unchecked();
        let value_str = symbol.value.format(false);
        let assignment_str = format!("{key_str}={value_str}");

        if let Some(line) = self.lines.get_mut(&line_idx) {
            line.push_str(&format!(" | {assignment_str}"));
        } else {
            self.lines
                .insert(line_idx, format!("[{prefix}] {assignment_str}"));
        }
    }
}

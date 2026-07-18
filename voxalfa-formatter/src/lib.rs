pub mod literal;

use std::collections::BTreeMap;

use voxalfa_validator::{
    ast::symbols::{SymbolRef, SymbolTree},
    diagnostic::Diagnostic,
    output::ValidatorOutput,
    render::RenderType,
    ts_utils::context::TSContext,
    validator::DocumentValidator,
};

use crate::literal::Formattable;

#[derive(Debug, Default)]
pub struct Formatter {
    lines: BTreeMap<usize, String>,
    separators: BTreeMap<usize, &'static str>,
    col_width: usize,
    col_factor: usize,
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

        self.col_width = output.resolve_column_width(RenderType::Text);
        self.col_factor = output.resolve_column_factor();

        self.process_header(&output);
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
        let value_str = symbol.value.format();
        let assignment_str = format!("{key_str}={value_str}");

        if let Some(line) = self.lines.get_mut(&line_idx) {
            line.push_str(&format!(" | {assignment_str}"));
        } else {
            self.lines
                .insert(line_idx, format!("[{prefix}] {assignment_str}"));
        }
    }
}

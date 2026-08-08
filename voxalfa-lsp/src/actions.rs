use std::collections::HashMap;

use async_lsp::lsp_types::{
    CodeAction, CodeActionKind, CodeActionOrCommand, Position, TextEdit, WorkspaceEdit,
};
use voxalfa_validator::{
    ast::symbols::{Scope, ScopeKind},
    ir::{PulseView, lyrics::LyricLineIr, solfa::SolfaLineIr},
    ts_utils::range::{Range, RangeUtil},
};

use crate::{
    state::Document,
    utils::{lsp_pos_to_ts, ts_range_to_lsp},
};

pub fn resolve_action_commands(
    document: &Document,
    position: Position,
) -> Option<Vec<CodeActionOrCommand>> {
    let position = lsp_pos_to_ts(&document.rope, position);
    let scope_id = document.data.symbols.query_scope(&position);
    let scope = document.data.symbols.get_scope(scope_id);
    let resolver = CodeActionResolver { doc: document };

    match scope.kind {
        ScopeKind::Pulse => resolver.get_pulse_actions(scope),
        _ => None,
    }
}

#[derive(Debug)]
struct CodeActionResolver<'a> {
    doc: &'a Document,
}

impl CodeActionResolver<'_> {
    fn get_pulse_actions(&self, pulse: &Scope) -> Option<Vec<CodeActionOrCommand>> {
        let symbols = &self.doc.data.symbols;
        let solfa = symbols.get_scope(pulse.parent?);
        let sub_section = symbols.get_scope(solfa.parent?);
        let section = symbols.get_scope(sub_section.parent?);

        if section.children.len() > 1 {
            return None;
        }

        let context = PulseContext {
            solfa,
            pulse,
            section,
            sub_section,
        };

        let top_merge_edits = self.make_top_pulse_merge_edits(&context);

        let action = top_merge_edits.map(|edits| CodeAction {
            title: "merge to previous section".to_string(),
            kind: Some(CodeActionKind::REFACTOR_REWRITE),
            edit: Some(WorkspaceEdit::new(HashMap::from([(
                self.doc.uri.clone(),
                edits,
            )]))),
            ..Default::default()
        })?;

        let result = vec![CodeActionOrCommand::CodeAction(action)];

        Some(result)
    }

    fn get_grouped_rows(
        &self,
        start: usize,
        end: usize,
        ctx: &PulseContext<'_>,
    ) -> Option<GroupedRows> {
        let section = &self.doc.data.body.sections[ctx.section.local_id];
        let sub_section = &section.items[ctx.sub_section.local_id];
        let view = &sub_section.views;

        let solfa = sub_section
            .solfa
            .iter()
            .map(|sol| self.resolve_source_pulse_range(start, end, sol))
            .collect::<Option<_>>()?;

        let lyrics = sub_section
            .lyrics
            .iter()
            .map(|lyr| self.resolve_source_lyrics_range(start, end, view, lyr))
            .collect::<Option<_>>()?;

        Some(GroupedRows { solfa, lyrics })
    }

    fn make_top_pulse_merge_edits(&self, ctx: &PulseContext<'_>) -> Option<Vec<TextEdit>> {
        if ctx.section.local_id < 1 {
            return None;
        }

        let rows = self.get_grouped_rows(0, ctx.pulse.local_id, ctx)?;
        let prev_section = &self.doc.data.body.sections[ctx.section.local_id - 1];
        let [prev_sub] = prev_section.items.as_array()?;

        if prev_sub.solfa.len() != rows.solfa.len() || prev_sub.lyrics.len() != rows.lyrics.len() {
            return None;
        }

        let mut result = Vec::new();

        for (row_id, line) in prev_sub.solfa.iter().enumerate() {
            let source_range = rows.solfa[row_id];
            let text = &self.doc.source[source_range.start_byte..source_range.end_byte];
            let fixed_text = format!("{text} ||");
            let mut target_range = self.doc.data.symbols.get_scope_range(line.sid).end();
            target_range.end_point.column -= 2; // before anchors
            let edits = self.make_cut_edits(fixed_text, source_range, target_range);

            result.extend(edits);
        }

        for (row_id, line) in prev_sub.lyrics.iter().enumerate() {
            let source_range = rows.lyrics[row_id];
            let text = &self.doc.source[source_range.start_byte..source_range.end_byte];
            let fixed_text = format!("{text} @@");
            let mut target_range = self.doc.data.symbols.get_scope_range(line.sid).end();
            target_range.end_point.column -= 2; // before anchors
            let edits = self.make_cut_edits(fixed_text, source_range, target_range);

            result.extend(edits);
        }

        // delete the entire section if empty
        if ctx.pulse.local_id == ctx.solfa.children.len() - 1 {
            let edit = self.make_delete_edit(ctx.section.range);
            let delimiters = self.doc.data.symbols.get_delimiters();

            let delimiter = delimiters
                .iter()
                .filter(|d| d.kind.is_section())
                .nth(ctx.section.local_id);

            result.push(edit);

            if let Some(delimiter) = delimiter {
                result.push(self.make_delete_edit(delimiter.range));
            }
        }

        Some(result)
    }

    fn resolve_source_pulse_range(
        &self,
        start: usize,
        end: usize,
        line: &SolfaLineIr,
    ) -> Option<Range> {
        let first = line.pulses.get(start)?;
        let first_range = self.doc.data.symbols.get_scope_range(first.sid);
        let last = line.pulses.get(end)?;
        let last_range = self.doc.data.symbols.get_scope_range(last.sid);

        Some(first_range.merge(last_range))
    }

    fn resolve_source_lyrics_range(
        &self,
        start: usize,
        end: usize,
        view: &[PulseView],
        line: &LyricLineIr,
    ) -> Option<Range> {
        let sub_view = view.get(start..=end)?;
        let col_start: usize = view.iter().take(start).map(|v| v.durations.len()).sum();
        let col_count: usize = sub_view.iter().map(|v| v.durations.len()).sum();
        let col_end = col_start + col_count;

        if col_count == 0 {
            return None;
        }

        let mut start_col_idx = None;
        let mut end_col_idx = None;
        let mut counter = 0;

        for (i, col) in line.columns.iter().enumerate() {
            let col_range_end = counter + col.span;

            if start_col_idx.is_none() && col_range_end > col_start {
                start_col_idx = Some(i);
            }

            if col_range_end >= col_end {
                end_col_idx = Some(i);
                break;
            }

            counter = col_range_end;
        }

        let start_index = start_col_idx?;
        let end_index = end_col_idx?;

        let start_col = line.columns.get(start_index)?;
        let start_range = self.doc.data.symbols.get_scope_range(start_col.sid);
        let end_col = line.columns.get(end_index)?;

        let end_range = match line.operators.get(end_index) {
            Some(op) => self.doc.data.symbols.get_symbol_range(op.sid).end(),
            _ => self.doc.data.symbols.get_scope_range(end_col.sid).end(),
        };

        Some(start_range.merge(end_range))
    }

    fn make_cut_edits(&self, text: String, source: Range, target: Range) -> [TextEdit; 2] {
        [
            self.make_paste_edit(text, target),
            self.make_delete_edit(source),
        ]
    }

    fn make_paste_edit(&self, text: String, target: Range) -> TextEdit {
        TextEdit {
            range: ts_range_to_lsp(&self.doc.rope, &target),
            new_text: text,
        }
    }

    fn make_delete_edit(&self, range: Range) -> TextEdit {
        TextEdit {
            range: ts_range_to_lsp(&self.doc.rope, &range),
            new_text: String::new(),
        }
    }
}

#[derive(Debug)]
struct PulseContext<'a> {
    solfa: &'a Scope,
    section: &'a Scope,
    sub_section: &'a Scope,
    pulse: &'a Scope,
}

#[derive(Debug, Default)]
struct GroupedRows {
    solfa: Vec<Range>,
    lyrics: Vec<Range>,
}

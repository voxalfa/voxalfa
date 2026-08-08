use std::collections::HashMap;

use async_lsp::lsp_types::{
    CodeAction, CodeActionKind, CodeActionOrCommand, Position, TextEdit, WorkspaceEdit,
};
use voxalfa_validator::{
    ast::symbols::{Scope, ScopeId, ScopeKind},
    ir::{PulseView, lyrics::LyricLineIr, solfa::SolfaLineIr},
    ts_utils::range::{Range, RangeUtil},
};

use crate::{
    state::Document,
    utils::{lsp_pos_to_ts, ts_range_to_lsp},
};

pub fn resolve_action_commands(
    doc: &Document,
    position: Position,
) -> Option<Vec<CodeActionOrCommand>> {
    let position = lsp_pos_to_ts(&doc.rope, position);
    let scope_id = doc.data.symbols.query_scope(&position);
    let scope = doc.data.symbols.get_scope(scope_id);
    let resolver = CodeActionResolver { doc };

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

        let has_assignment = sub_section
            .children
            .iter()
            .chain(&section.children)
            .map(|&s| self.doc.data.symbols.get_scope(s))
            .any(|c| c.kind.is_assignemnt()); // TODO: resolve params?

        if section.children.len() > 1 || has_assignment {
            return None;
        }

        let context = PulseContext {
            solfa,
            pulse,
            section,
            sub_section,
        };

        self.build_pulse_actions(context)
    }

    fn build_pulse_actions(&self, ctx: PulseContext<'_>) -> Option<Vec<CodeActionOrCommand>> {
        let mut result = Vec::new();

        if let Some(action) = self.make_top_merge_action(&ctx) {
            result.push(action);
        }

        if let Some(action) = self.make_bottom_merge_action(&ctx) {
            result.push(action);
        }

        if result.is_empty() {
            None
        } else {
            Some(result)
        }
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

    fn make_top_merge_action(&self, ctx: &PulseContext<'_>) -> Option<CodeActionOrCommand> {
        if ctx.section.local_id < 1 {
            return None;
        }

        let rows = self.get_grouped_rows(0, ctx.pulse.local_id, ctx)?;
        let prev_section = &self.doc.data.body.sections[ctx.section.local_id - 1];
        let [prev_sub] = prev_section.items.as_array()?;

        if prev_sub.solfa.len() != rows.solfa.len() || prev_sub.lyrics.len() != rows.lyrics.len() {
            return None;
        }

        let mut edits = Vec::new();

        let solfa_scopes = prev_sub.solfa.iter().map(|s| s.sid);
        let lyrics_scopes = prev_sub.lyrics.iter().map(|s| s.sid);

        self.apply_top_merge_edits(&rows.solfa, solfa_scopes, &mut edits);
        self.apply_top_merge_edits(&rows.lyrics, lyrics_scopes, &mut edits);

        if ctx.pulse.local_id == ctx.solfa.children.len() - 1 {
            self.apply_section_delete_edits(ctx, 0, &mut edits);
        }

        let action = self.make_code_action(
            "merge to top section",
            CodeActionKind::REFACTOR_REWRITE,
            edits,
        );

        Some(action)
    }

    fn make_bottom_merge_action(&self, ctx: &PulseContext<'_>) -> Option<CodeActionOrCommand> {
        let total_sections = self.doc.data.body.sections.len();

        if ctx.section.local_id + 1 >= total_sections {
            return None;
        }

        let last_pulse_idx = ctx.solfa.children.len().checked_sub(1)?;
        let rows = self.get_grouped_rows(ctx.pulse.local_id, last_pulse_idx, ctx)?;
        let next_section = &self.doc.data.body.sections[ctx.section.local_id + 1];
        let [next_sub] = next_section.items.as_array()?;
        let symbols = &self.doc.data.symbols;

        if next_sub.solfa.len() != rows.solfa.len() || next_sub.lyrics.len() != rows.lyrics.len() {
            return None;
        }

        let mut edits = Vec::new();

        for (row_id, line) in next_sub.solfa.iter().enumerate() {
            let source_range = rows.solfa[row_id];
            let text = &self.doc.source[source_range.start_byte..source_range.end_byte];
            let first_pulse = line.pulses.first()?;
            let target_range = symbols.get_scope_range(first_pulse.sid).start();
            let cut_edit = self.make_cut_edits(text, source_range, target_range);

            edits.extend(cut_edit);
        }

        for (row_id, line) in next_sub.lyrics.iter().enumerate() {
            let source_range = rows.lyrics[row_id];
            let text = &self.doc.source[source_range.start_byte..source_range.end_byte];
            let first_column = line.columns.first()?;
            let target_range = symbols.get_scope_range(first_column.sid).start();
            let cut_edit = self.make_cut_edits(text, source_range, target_range);

            edits.extend(cut_edit);
        }

        if ctx.pulse.local_id == 0 {
            self.apply_section_delete_edits(ctx, 1, &mut edits);
        }

        let action = self.make_code_action(
            "merge to bottom section",
            CodeActionKind::REFACTOR_REWRITE,
            edits,
        );

        Some(action)
    }

    fn apply_top_merge_edits<I>(&self, rows: &[Range], scopes: I, edits: &mut Vec<TextEdit>)
    where
        I: Iterator<Item = ScopeId>,
    {
        for (row_id, sid) in scopes.enumerate() {
            let source_range = rows[row_id];
            let text = &self.doc.source[source_range.start_byte..source_range.end_byte];
            let mut target_range = self.doc.data.symbols.get_scope_range(sid).end();

            // insert items before the delimiters '||' and '@@'
            target_range.start_point.column = target_range.start_point.column.saturating_sub(2);
            target_range.end_point.column = target_range.end_point.column.saturating_sub(2);

            let cut_edit = self.make_cut_edits(text, source_range, target_range);

            edits.extend(cut_edit);
        }
    }

    fn apply_section_delete_edits(
        &self,
        ctx: &PulseContext<'_>,
        section_offset: usize,
        edits: &mut Vec<TextEdit>,
    ) {
        let edit = self.make_delete_edit(ctx.section.range);
        let delimiters = self.doc.data.symbols.get_delimiters();
        let delimiter_id = ctx.section.local_id.saturating_sub(section_offset);

        let delimiter = delimiters
            .iter()
            .filter(|d| d.kind.is_section())
            .nth(delimiter_id);

        edits.push(edit);

        if let Some(delimiter) = delimiter {
            edits.push(self.make_delete_edit(delimiter.range));
        }
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

    fn make_code_action(
        &self,
        title: &str,
        kind: CodeActionKind,
        edits: Vec<TextEdit>,
    ) -> CodeActionOrCommand {
        let edit = HashMap::from([(self.doc.uri.clone(), edits)]);

        let action = CodeAction {
            title: title.to_string(),
            kind: Some(kind),
            edit: Some(WorkspaceEdit::new(edit)),
            ..Default::default()
        };

        CodeActionOrCommand::CodeAction(action)
    }

    fn make_cut_edits(&self, text: &str, source: Range, target: Range) -> [TextEdit; 2] {
        [
            self.make_paste_edit(text, target),
            self.make_delete_edit(source),
        ]
    }

    fn make_paste_edit(&self, text: &str, target: Range) -> TextEdit {
        TextEdit {
            range: ts_range_to_lsp(&self.doc.rope, &target),
            new_text: text.to_string(),
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

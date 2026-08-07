use std::collections::HashMap;

use async_lsp::lsp_types::{
    CodeAction, CodeActionKind, CodeActionOrCommand, TextEdit, Url, WorkspaceEdit,
};
use voxalfa_validator::{
    ast::symbols::{Scope, ScopeKind, SymbolTree},
    ts_utils::range::{Position, Range, RangeUtil},
};

use crate::{state::Document, utils::ts_range_to_lsp};

pub fn resolve_action_commands(
    uri: Url,
    document: &Document,
    position: Position,
) -> Option<Vec<CodeActionOrCommand>> {
    let scope_id = document.data.symbols.query_scope(&position);
    let scope = document.data.symbols.get_scope(scope_id);

    let resolver = CodeActionResolver {
        uri,
        source: &document.source,
        tree: &document.data.symbols,
    };

    match scope.kind {
        ScopeKind::Pulse => resolver.get_pulse_actions(scope),
        _ => None,
    }
}

#[derive(Debug)]
struct CodeActionResolver<'a> {
    uri: Url,
    tree: &'a SymbolTree,
    source: &'a str,
}

impl CodeActionResolver<'_> {
    fn get_pulse_actions(&self, pulse: &Scope) -> Option<Vec<CodeActionOrCommand>> {
        let solfa = self.tree.get_scope(pulse.parent?);
        let sub_section = self.tree.get_scope(solfa.parent?);
        let section = self.tree.get_scope(sub_section.parent?);
        let body = self.tree.get_scope(section.parent?);

        if section.children.len() > 1 {
            return None;
        }

        let rows = sub_section
            .children
            .iter()
            .map(|&sid| self.tree.get_scope(sid))
            .flat_map(|solfa| {
                let first = solfa.children.get(pulse.local_id)?;
                let first_range = self.tree.get_scope_range(*first);
                Some(first_range.merge(solfa.range))
            })
            .collect::<Vec<_>>();

        if rows.len() != sub_section.children.len() {
            return None;
        }

        let context = PulseContext {
            pulse,
            section,
            body,
            rows,
        };

        let action = self
            .get_prev_section_pulse_edits(context)
            .map(|edits| CodeAction {
                title: "merge to previous section".to_string(),
                kind: Some(CodeActionKind::REFACTOR_REWRITE),
                edit: Some(WorkspaceEdit::new(HashMap::from([(
                    self.uri.clone(),
                    edits,
                )]))),
                ..Default::default()
            })?;

        let result = vec![CodeActionOrCommand::CodeAction(action)];

        Some(result)
    }

    fn get_prev_section_pulse_edits(&self, ctx: PulseContext<'_>) -> Option<Vec<TextEdit>> {
        if ctx.section.local_id < 1 || ctx.pulse.local_id > 0 {
            return None;
        }

        let prev_sid = ctx.body.children[ctx.section.local_id - 1];
        let prev_section = self.tree.get_scope(prev_sid);

        let [prev_sub_sid] = prev_section.children.as_array()?;
        let prev_sub = self.tree.get_scope(*prev_sub_sid);

        if prev_sub.children.len() != ctx.rows.len() {
            return None;
        }

        let mut result = Vec::new();

        for (row_id, line_sid) in prev_sub.children.iter().enumerate() {
            let line = self.tree.get_scope(*line_sid);
            let source_range = ctx.rows[row_id];
            let mut target_range = line.range.end();
            target_range.end_point.column -= 2;
            let text = &self.source[source_range.start_byte..source_range.end_byte];

            result.extend([
                TextEdit {
                    range: ts_range_to_lsp(&target_range),
                    new_text: text.to_string(),
                },
                TextEdit {
                    range: ts_range_to_lsp(&source_range),
                    new_text: String::new(), // delete the source row
                },
            ]);
        }

        let section_delimiter = self
            .tree
            .get_delimiters()
            .iter()
            .filter(|d| d.kind.is_section())
            .nth(ctx.section.local_id);

        if let Some(delimiter) = section_delimiter {
            result.push(TextEdit {
                range: ts_range_to_lsp(&delimiter.range),
                new_text: String::new(), // delete the delimiter
            });
        }

        result.push(TextEdit {
            range: ts_range_to_lsp(&ctx.section.range),
            new_text: String::new(), // delete the section
        });

        Some(result)
    }
}

#[derive(Debug)]
struct PulseContext<'a> {
    pulse: &'a Scope,
    section: &'a Scope,
    body: &'a Scope,
    rows: Vec<Range>,
}

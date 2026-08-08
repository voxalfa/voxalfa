use async_lsp::lsp_types::Position;
use voxalfa_core::ast::symbols::SymbolKind;

use crate::{
    builtin::VOICE_BUILTINS,
    parameters::{HEADER_PARAMS, INITIAL_PARAMS, ParamSpec, SECTION_PARAMS},
    state::Document,
    utils::lsp_pos_to_ts,
};

pub fn find_param(key: &str) -> Option<&'static ParamSpec> {
    [HEADER_PARAMS, INITIAL_PARAMS, SECTION_PARAMS]
        .iter()
        .flat_map(|slice| slice.iter())
        .find(|param| param.name == key)
}

pub fn create_documentation(doc: &Document, position: Position) -> Option<String> {
    let position = lsp_pos_to_ts(&doc.rope, position);
    let symbol = doc.data.symbols.query_symbol(&position)?;

    match &symbol.kind {
        SymbolKind::Key(key) => {
            let spec = find_param(key.as_str())?;
            Some(format!(
                "```text\n{}: {}\n```\n---\n{}",
                spec.name, spec.type_str, spec.doc
            ))
        }
        SymbolKind::Value(value) => Some(format!("```text\n{}\n```", value)),
        SymbolKind::Voice(voice_id) => {
            let voice = doc.data.symbols.get_voice(*voice_id)?;
            let voice_str = voice.to_string();

            VOICE_BUILTINS
                .iter()
                .find(|v| v.label == voice_str)
                .map(|spec| {
                    format!(
                        "```text\n{} ({})\n```\n---\n{}",
                        spec.label, spec.detail, spec.doc
                    )
                })
        }
        _ => None,
    }
}

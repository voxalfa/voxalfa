use async_lsp::lsp_types::{GotoDefinitionResponse, Location, Position};
use voxalfa_core::ast::symbols::SymbolKind;

use crate::{
    parameters::INITIAL_PARAMS,
    state::Document,
    utils::{lsp_pos_to_ts, ts_range_to_lsp},
};

pub fn resolve_symbol_definition(
    doc: &Document,
    position: Position,
) -> Option<GotoDefinitionResponse> {
    let position = lsp_pos_to_ts(&doc.rope, position);
    let symbol = doc.data.symbols.query_symbol(&position)?;
    let params = &doc.data.header.params;

    let range = match &symbol.kind {
        SymbolKind::Key(key) if INITIAL_PARAMS.iter().any(|p| p.name == key) => {
            doc.data.symbols.get_key_definition(key)
        }
        SymbolKind::Voice(voice) => params
            .voices
            .as_ref()
            .and_then(|v| v.value.iter().enumerate().find(|(id, _)| id == voice))
            .map(|(_, v)| doc.data.symbols.get_symbol_range(v.sid)),
        _ => None,
    };

    range.map(|r| {
        GotoDefinitionResponse::Scalar(Location {
            uri: doc.uri.clone(),
            range: ts_range_to_lsp(&doc.rope, &r),
        })
    })
}

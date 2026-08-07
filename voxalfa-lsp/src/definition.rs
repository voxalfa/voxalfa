use async_lsp::lsp_types::{GotoDefinitionResponse, Location, Url};
use voxalfa_validator::{
    ast::symbols::{Symbol, SymbolKind},
    output::FinalOutput,
};

use crate::{parameters::INITIAL_PARAMS, utils::convert_range};

pub fn resolve_symbol_definition(
    uri: Url,
    symbol: &Symbol,
    data: &FinalOutput,
) -> Option<GotoDefinitionResponse> {
    let params = &data.header.params;

    let range = match &symbol.kind {
        SymbolKind::Key(key) if INITIAL_PARAMS.iter().any(|p| p.name == key) => {
            data.symbols.get_key_definition(key)
        }
        SymbolKind::Voice(voice) => params
            .voices
            .as_ref()
            .and_then(|v| v.value.iter().enumerate().find(|(id, _)| id == voice))
            .map(|(_, v)| data.symbols.get_symbol_range(v.sid)),
        _ => None,
    };

    range.map(|r| {
        GotoDefinitionResponse::Scalar(Location {
            uri,
            range: convert_range(&r),
        })
    })
}

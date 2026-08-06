use async_lsp::lsp_types::{DocumentSymbol, DocumentSymbolResponse, SymbolKind as LspSymbolKind};

use voxalfa_validator::ast::symbols::{
    Primitive, ROOT_SCOPE_ID, Scope, ScopeKind, Symbol, SymbolKind, SymbolTree, Value,
};

use crate::utils::convert_range;

pub fn resolve_document_symbols(tree: &SymbolTree) -> Option<DocumentSymbolResponse> {
    let root_scope = tree.get_scope(ROOT_SCOPE_ID);
    let children = resolve_scope_children(tree, root_scope);

    if children.is_empty() {
        None
    } else {
        Some(DocumentSymbolResponse::Nested(children))
    }
}

fn resolve_scope_children(tree: &SymbolTree, scope: &Scope) -> Vec<DocumentSymbol> {
    let mut symbols = Vec::new();

    for &symbol_id in &scope.symbols {
        let symbol = tree.get_symbol(symbol_id);
        if let Some(doc_sym) = convert_symbol(symbol) {
            symbols.push(doc_sym);
        }
    }

    for &child_scope_id in &scope.children {
        let child_scope = tree.get_scope(child_scope_id);
        if let Some(doc_sym) = convert_scope(tree, child_scope) {
            symbols.push(doc_sym);
        }
    }

    symbols
}

fn convert_scope(tree: &SymbolTree, scope: &Scope) -> Option<DocumentSymbol> {
    if scope.kind.is_hidden() {
        return None;
    }

    let children = resolve_scope_children(tree, scope);
    let name = format!("{:?}", scope.kind);
    let kind = map_scope_kind(&scope.kind);
    let range = convert_range(&scope.range);

    #[allow(deprecated)]
    Some(DocumentSymbol {
        name,
        detail: Some(format!("{:?}", scope.kind)),
        kind,
        tags: None,
        deprecated: None,
        range,
        selection_range: range,
        children: if children.is_empty() {
            None
        } else {
            Some(children)
        },
    })
}

fn convert_symbol(symbol: &Symbol) -> Option<DocumentSymbol> {
    let (name, detail, kind) = match &symbol.kind {
        SymbolKind::Key(key) => (
            key.clone(),
            Some("Key".to_string()),
            LspSymbolKind::PROPERTY,
        ),
        SymbolKind::Value(val) => (val.to_string(), None, map_value_kind(val)),
        SymbolKind::Voice(voice) => (
            format!("{voice:?}"),
            Some("Voice".to_string()),
            LspSymbolKind::VARIABLE,
        ),
        SymbolKind::Token => return None,
    };

    let range = convert_range(&symbol.range);

    #[allow(deprecated)]
    Some(DocumentSymbol {
        name,
        detail,
        kind,
        tags: None,
        deprecated: None,
        range,
        selection_range: range,
        children: None,
    })
}

fn map_scope_kind(kind: &ScopeKind) -> LspSymbolKind {
    match kind {
        ScopeKind::Root | ScopeKind::Body => LspSymbolKind::MODULE,
        ScopeKind::Header => LspSymbolKind::NAMESPACE,
        ScopeKind::DirectiveLine => LspSymbolKind::OPERATOR,
        ScopeKind::AssignmentLine | ScopeKind::Assignment => LspSymbolKind::VARIABLE,
        ScopeKind::Section | ScopeKind::SubSection => LspSymbolKind::CLASS,
        ScopeKind::SolfaLine | ScopeKind::Pulse => LspSymbolKind::STRING,
        ScopeKind::LyricLine | ScopeKind::LyricsColumn | ScopeKind::LyricString => {
            LspSymbolKind::STRING
        }
        ScopeKind::List => LspSymbolKind::ARRAY,
    }
}

fn map_value_kind(val: &Value) -> LspSymbolKind {
    match val {
        Value::Primitive(prim) => match prim {
            Primitive::String => LspSymbolKind::STRING,
            Primitive::Integer | Primitive::Float => LspSymbolKind::NUMBER,
            Primitive::Boolean => LspSymbolKind::BOOLEAN,
            Primitive::Voice => LspSymbolKind::VARIABLE,
            _ => LspSymbolKind::ENUM_MEMBER,
        },
        Value::List(_) => LspSymbolKind::ARRAY,
    }
}

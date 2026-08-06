use std::collections::HashMap;

use crate::{
    ast::parser::Parser,
    data_types::Voice,
    ts_utils::{
        range::{Position, Range, RangeUtil},
        types::AssignmentData,
    },
};

pub const ROOT_SCOPE_ID: usize = 0;

pub type SymbolId = usize;
pub type ScopeId = usize;
pub type LyricStringId = usize;
pub type Comment = SymbolRef<String>;
pub type Field<T> = Option<SymbolRef<T>>;

#[derive(Debug, Clone)]
pub struct SymbolRef<T> {
    pub sid: SymbolId,
    pub value: T,
}

pub trait FieldAssign {
    fn assign_field(&mut self, data: AssignmentData, context: &mut Parser);
}

#[derive(Debug)]
pub enum SymbolKind {
    Key(String),
    Value(Value),
    Voice(Voice),
    Token,
}

impl SymbolKind {
    pub fn primitive_value(primitive: Primitive) -> Self {
        SymbolKind::Value(Value::Primitive(primitive))
    }

    pub fn as_key_unchecked(&self) -> &str {
        match self {
            SymbolKind::Key(key) => key,
            _ => unreachable!("invalid key symbol"),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Primitive {
    String,
    Integer,
    Float,
    Boolean,
    Key,
    Mark,
    Dynamic,
    Jump,
    Tempo,
    Voice,
    Touch,
}

impl std::fmt::Display for Primitive {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", format!("{self:?}").to_lowercase())
    }
}

#[derive(Debug)]
pub enum Value {
    Primitive(Primitive),
    List(Primitive),
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Primitive(primitive) => write!(f, "{primitive}"),
            Value::List(primitive) => write!(f, "{{{primitive}...}}"),
        }
    }
}

#[derive(Debug)]
pub struct Symbol {
    pub range: Range,
    pub kind: SymbolKind,
    pub scope: ScopeId,
}

#[derive(Debug)]
pub enum ScopeKind {
    Root,
    Header,
    DirectiveLine,
    AssignmentLine,
    Assignment,
    Body,
    Section,
    SubSection,
    SolfaLine,
    Pulse,
    LyricLine,
    LyricsColumn,
    LyricString,
    List,
}

#[derive(Debug)]
pub struct Scope {
    pub range: Range,
    pub kind: ScopeKind,
    pub parent: Option<ScopeId>,
    pub children: Vec<ScopeId>,
    pub symbols: Vec<SymbolId>,
}

#[derive(Debug, Default)]
pub struct SymbolCache {
    key_refs: HashMap<String, Vec<SymbolId>>,
    voice_refs: HashMap<Voice, Vec<SymbolId>>,
    comments: Vec<Comment>,
    lyrics: Vec<String>,
}

#[derive(Debug, Default)]
pub struct SymbolTree {
    symbols: Vec<Symbol>,
    scopes: Vec<Scope>,
    cache: SymbolCache,
}

impl SymbolTree {
    pub fn init_root(&mut self, range: Range) {
        self.add_scope(ScopeKind::Root, range, None);
    }

    pub fn add_symbol(&mut self, kind: SymbolKind, range: Range, scope_id: ScopeId) -> SymbolId {
        let id = self.symbols.len();

        let symbol = Symbol {
            range,
            kind,
            scope: scope_id,
        };

        self.symbols.push(symbol);
        self.scopes[scope_id].symbols.push(id);

        id
    }

    pub fn get_symbol(&self, id: SymbolId) -> &Symbol {
        &self.symbols[id]
    }

    pub fn get_symbol_range(&self, id: SymbolId) -> Range {
        self.symbols[id].range
    }

    pub fn add_scope(&mut self, kind: ScopeKind, range: Range, parent: Option<ScopeId>) -> ScopeId {
        let id = self.scopes.len();

        let scope = Scope {
            range,
            kind,
            parent,
            children: Vec::new(),
            symbols: Vec::new(),
        };

        if let Some(parent) = parent {
            self.scopes[parent].children.push(id);
        }

        self.scopes.push(scope);

        id
    }

    pub fn get_scope(&self, id: ScopeId) -> &Scope {
        &self.scopes[id]
    }

    pub fn get_scope_range(&self, id: ScopeId) -> Range {
        self.scopes[id].range
    }

    pub fn resolve_scope(&self, symbol_id: SymbolId) -> &Scope {
        let scope_id = self.symbols[symbol_id].scope;
        &self.scopes[scope_id]
    }

    pub fn store_comment(&mut self, comment: SymbolRef<String>) {
        self.cache.comments.push(comment);
    }

    pub fn get_comments(&self) -> &[SymbolRef<String>] {
        &self.cache.comments
    }

    pub fn store_lyric_chunk(&mut self, chunk: String) -> LyricStringId {
        self.cache.lyrics.push(chunk);
        self.cache.lyrics.len().saturating_sub(1)
    }

    pub fn get_lyric_chunk(&self, id: LyricStringId) -> &str {
        &self.cache.lyrics[id]
    }

    pub fn store_key_ref(&mut self, key: String, sid: SymbolId) {
        self.cache.key_refs.entry(key).or_default().push(sid);
    }

    pub fn store_voice_ref(&mut self, voice: Voice, sid: SymbolId) {
        self.cache.voice_refs.entry(voice).or_default().push(sid);
    }

    pub fn get_symbol_refs(&self, position: &Position) -> Option<Vec<Range>> {
        let symbol = self.query_symbol(position)?;

        let symbols = match &symbol.kind {
            SymbolKind::Key(key) => self.cache.key_refs.get(key),
            SymbolKind::Voice(voice) => self.cache.voice_refs.get(voice),
            _ => None,
        };

        symbols.map(|v| v.iter().map(|&sid| self.symbols[sid].range).collect())
    }

    pub fn query_symbol(&self, position: &Position) -> Option<&Symbol> {
        let scope_id = self.query_scope(position);

        self.scopes[scope_id]
            .symbols
            .iter()
            .find(|&sid| self.symbols[*sid].range.contains(position))
            .map(|&sid| &self.symbols[sid])
    }

    pub fn query_scope(&self, position: &Position) -> ScopeId {
        let mut current_id = ROOT_SCOPE_ID;

        while let Some(&child_id) = self.scopes[current_id]
            .children
            .iter()
            .find(|&&cid| self.scopes[cid].range.contains(position))
        {
            current_id = child_id;
        }

        current_id
    }
}

#[derive(Debug)]
pub struct Delimiter {
    pub kind: DelimiterKind,
    pub line: usize,
}

#[derive(Debug)]
pub enum DelimiterKind {
    Header,
    SectionSplit,
    SectionMajor,
    SectionMerge,
    SubSection,
}

impl std::fmt::Display for DelimiterKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DelimiterKind::Header => write!(f, "---"),
            DelimiterKind::SectionSplit => write!(f, "--"),
            DelimiterKind::SectionMajor => write!(f, "=="),
            DelimiterKind::SectionMerge => write!(f, "<<"),
            DelimiterKind::SubSection => write!(f, "++"),
        }
    }
}

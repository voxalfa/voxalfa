use std::collections::HashMap;

use tree_sitter::Point;

use crate::{
    ast::parser::Parser,
    data_types::Voice,
    ts_utils::{
        range::{Range, RangeUtil},
        types::AssignmentData,
    },
};

pub const ROOT_SCOPE_ID: usize = 0;

pub type SymbolId = usize;
pub type ScopeId = usize;
pub type VoiceId = usize;
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
    Voice(VoiceId),
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

impl ScopeKind {
    pub fn is_hidden(&self) -> bool {
        matches!(
            self,
            ScopeKind::DirectiveLine | ScopeKind::List | ScopeKind::LyricString
        )
    }

    pub fn is_solfa_line(&self) -> bool {
        matches!(self, Self::SolfaLine)
    }
}

#[derive(Debug)]
pub struct Scope {
    pub local_id: usize,
    pub range: Range,
    pub kind: ScopeKind,
    pub parent: Option<ScopeId>,
    pub children: Vec<ScopeId>,
    pub symbols: Vec<SymbolId>,
}

#[derive(Debug, Default)]
pub struct SymbolCache {
    key_refs: HashMap<String, Vec<SymbolId>>,
    voice_defs: Vec<Voice>,
    voice_refs: HashMap<VoiceId, Vec<SymbolId>>,
    comments: Vec<Comment>,
    lyrics: Vec<String>,
    delimiters: Vec<Delimiter>,
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

        let local_id = parent
            .map(|p| self.get_scope(p))
            .map(|s| s.children.len())
            .unwrap_or_default();

        let scope = Scope {
            local_id,
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

    pub fn define_voices(&mut self, voices: &[SymbolRef<Voice>]) {
        for (id, voice) in voices.iter().enumerate() {
            self.store_voice_ref(id, voice.sid);
            self.cache.voice_defs.push(voice.value);
        }
    }

    pub fn store_voice_ref(&mut self, voice: VoiceId, sid: SymbolId) {
        self.cache.voice_refs.entry(voice).or_default().push(sid);
    }

    pub fn get_voice(&self, voice_id: usize) -> Option<Voice> {
        self.cache.voice_defs.get(voice_id).copied()
    }

    pub fn get_symbol_refs(&self, position: &Point) -> Option<Vec<Range>> {
        let symbol = self.query_symbol(position)?;

        let symbols = match &symbol.kind {
            SymbolKind::Key(key) => self.cache.key_refs.get(key),
            SymbolKind::Voice(voice) => self.cache.voice_refs.get(voice),
            SymbolKind::Value(Value::Primitive(Primitive::Voice)) => self.find_voice_refs(position),
            _ => None,
        };

        symbols.map(|v| v.iter().map(|&sid| self.symbols[sid].range).collect())
    }

    pub fn get_key_definition(&self, key: &str) -> Option<Range> {
        self.cache
            .key_refs
            .get(key)
            .and_then(|refs| refs.first().map(|sid| self.symbols[*sid].range))
    }

    pub fn query_symbol(&self, position: &Point) -> Option<&Symbol> {
        let scope_id = self.query_scope(position);

        self.scopes[scope_id]
            .symbols
            .iter()
            .find(|&sid| self.symbols[*sid].range.contains(position))
            .map(|&sid| &self.symbols[sid])
    }

    pub fn query_scope(&self, position: &Point) -> ScopeId {
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

    pub fn create_voice_id(&self, line_sid: ScopeId) -> usize {
        let sub_section_sid = self.scopes[line_sid].parent.unwrap();
        let section_sid = self.scopes[sub_section_sid].parent.unwrap();

        let line_count = self.scopes[section_sid]
            .children
            .iter()
            .flat_map(|&sub_sid| &self.scopes[sub_sid].children)
            .filter(|&&sid| matches!(self.scopes[sid].kind, ScopeKind::SolfaLine))
            .count();

        line_count - 1
    }

    pub fn find_voice_refs(&self, position: &Point) -> Option<&Vec<SymbolId>> {
        let voice = self.cache.voice_refs.iter().find_map(|(v, refs)| {
            refs.iter()
                .any(|&sid| self.symbols[sid].range.contains(position))
                .then_some(v)
                .copied()
        });

        voice.and_then(|v| self.cache.voice_refs.get(&v))
    }

    pub fn store_delimiter(&mut self, range: Range, kind: DelimiterKind) {
        self.cache.delimiters.push(Delimiter { kind, range });
    }

    pub fn get_delimiters(&self) -> &[Delimiter] {
        &self.cache.delimiters
    }
}

#[derive(Debug)]
pub struct Delimiter {
    pub kind: DelimiterKind,
    pub range: Range,
}

#[derive(Debug)]
pub enum DelimiterKind {
    Header,
    SectionSplit,
    SectionMajor,
    SectionMerge,
    SubSection,
}

impl DelimiterKind {
    pub fn is_section(&self) -> bool {
        matches!(
            self,
            DelimiterKind::SectionSplit | DelimiterKind::SectionMajor | DelimiterKind::SectionMerge
        )
    }
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

use crate::{
    ts_utils::{range::Range, types::AssignmentData},
    validator::DocumentValidator,
};

pub type SymbolId = usize;
pub type ScopeId = usize;

#[derive(Debug)]
pub enum SymbolKind {
    Key(String),
    Value(Value),
    Comment(Comment),
    Token,
}

#[derive(Debug)]
pub enum Value {
    String,
    Integer,
    Float,
    Boolean,
    Builtin,
    List,
}

#[derive(Debug)]
pub enum Comment {
    Inline,
}

#[derive(Debug)]
pub struct Symbol {
    pub range: Range,
    pub kind: SymbolKind,
    pub scope: ScopeId,
}

#[derive(Debug)]
pub struct SymbolRef<T> {
    pub sid: SymbolId,
    pub value: T,
}

#[derive(Debug)]
pub enum ScopeKind {
    Header,
    AssignmentLine,
    Assignment,
    Body,
    Section,
    SolfaLine,
    Pulse,
    LyricLine,
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
pub struct SymbolTree {
    pub symbols: Vec<Symbol>,
    pub scopes: Vec<Scope>,
}

impl SymbolTree {
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
}

pub type Field<T> = Option<SymbolRef<T>>;

pub trait FieldAssign {
    fn assign_field(&mut self, data: AssignmentData, context: &mut DocumentValidator);
}

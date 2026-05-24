use crate::ts_utils::range::Range;

// pub type Field<T> = Option<Assignment<T>>;

// pub trait FieldAssign {
//     fn assign_field(&mut self, source: AssignmentDataSource, context: &mut DocumentValidator);
// }

// #[derive(Debug, Default)]
// pub struct Document {
//     pub header: Header,
//     pub body: Body,
// }
//
// impl Document {
//     pub fn get_voice(&self, id: usize) -> Option<Voice> {
//         self.header
//             .params
//             .voices
//             .as_ref()
//             .and_then(|v| v.value.get(id))
//             .copied()
//     }
// }
//
// #[derive(Debug, Default)]
// pub struct Header {
//     pub metadata: HeaderMetadata,
//     pub params: CompositionParams,
// }
//
// #[derive(Debug, Default)]
// pub struct HeaderMetadata {
//     pub title: Field<String>,
//     pub author: Field<Vec<String>>,
//     pub composer: Field<Vec<String>>,
//     pub release: Field<usize>,
//     pub description: Field<String>,
// }
//
// impl FieldAssign for HeaderMetadata {
//     fn assign_field(&mut self, source: AssignmentDataSource, context: &mut DocumentValidator) {
//         match source.data.key.name.as_str() {
//             "title" => context.assign_field(source, &mut self.title),
//             "author" => context.assign_field(source, &mut self.author),
//             "composer" => context.assign_field(source, &mut self.composer),
//             "release" => context.assign_field(source, &mut self.release),
//             "description" => context.assign_field(source, &mut self.description),
//             _ => {}
//         }
//     }
// }
//
// #[derive(Debug, Default)]
// pub struct CompositionParams {
//     pub key: Field<Key>,
//     pub time: Field<TimeSignature>,
//     pub bpm: Field<usize>,
//     pub voices: Field<Vec<Voice>>,
// }
//
// impl FieldAssign for CompositionParams {
//     fn assign_field(&mut self, source: AssignmentDataSource, context: &mut DocumentValidator) {
//         match source.data.key.name.as_str() {
//             "key" => context.assign_field(source, &mut self.key),
//             "time" => context.assign_field(source, &mut self.time),
//             "bpm" => context.assign_field(source, &mut self.bpm),
//             "voices" => context.assign_field(source, &mut self.voices),
//             _ => {
//                 context.report_error(
//                     source.data.range,
//                     DiagnosticKind::UnknownParameter(source.data.key.name.clone()),
//                 );
//             }
//         }
//     }
// }
//
// #[derive(Debug, Default)]
// pub struct SectionParams {
//     pub base: CompositionParams,
//     pub repeat: Field<bool>,
// }
//
// impl FieldAssign for SectionParams {
//     fn assign_field(&mut self, source: AssignmentDataSource, context: &mut DocumentValidator) {
//         match source.data.key.name.as_str() {
//             "repeat" => context.assign_field(source, &mut self.repeat),
//             "voices" => {
//                 context.report_error(
//                     source.data.range,
//                     DiagnosticKind::UnknownParameter(source.data.key.name.clone()),
//                 ); // do not allow voice override inside sections
//             }
//             _ => self.base.assign_field(source, context),
//         }
//     }
// }
//
// #[derive(Debug, Default)]
// pub struct Body {
//     pub sections: Vec<Section>,
// }
//
// #[derive(Debug, Default)]
// pub struct Section {
//     pub params: SectionParams,
//     pub dynamics: Dynamics,
//     pub solfa: Vec<SolfaLine>,
//     pub lyrics: Vec<LyricLine>,
// }
//
// #[derive(Debug, Default)]
// pub struct Dynamics {
//     pub value: Vec<Assignment<Dynamic>>,
// }
//
// impl FieldAssign for Dynamics {
//     fn assign_field(&mut self, source: AssignmentDataSource, context: &mut DocumentValidator) {
//         let name = source.data.key.name.clone();
//
//         if let Ok(kind) = DynamicKind::try_from(name.as_str()) {
//             let params = context
//                 .parse_node::<Vec<_>>(source.value_node)
//                 .unwrap_or_default();
//
//             let expected_params = match kind {
//                 DynamicKind::Cre | DynamicKind::Dec => 2,
//                 _ => 1,
//             };
//
//             if params.len() != expected_params {
//                 context.report_error(
//                     source.data.value.range,
//                     DiagnosticKind::InvalidDynamicParams(expected_params),
//                 );
//             } else {
//                 let assignment = Assignment {
//                     value: Dynamic {
//                         kind,
//                         start: params[0],
//                         end: *params.get(1).unwrap_or(&params[0]),
//                     },
//                     data: source.data,
//                 };
//
//                 self.value.push(assignment);
//             }
//         } else {
//             context.report_error(
//                 source.data.range,
//                 DiagnosticKind::InvalidDynamic(name.clone()),
//             );
//         }
//     }
// }

pub type SymbolId = usize;
pub type ScopeId = usize;

#[derive(Debug)]
pub enum SymbolKind {
    Key(String),
    Value(Value),
    Comment(Comment),
}

#[derive(Debug)]
pub enum Value {
    String,
    Integer,
    Float,
    Boolean,
    Token,
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
    pub id: SymbolId,
    pub value: T,
}

pub type Field<T> = Option<SymbolRef<T>>;

#[derive(Debug)]
pub enum ScopeKind {
    Header,
    AssignmentLine,
    Assignment,
    Body,
    Section,
    SolfaLine,
    Measure,
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

    pub fn get_scope(&self, id: SymbolId) -> &Scope {
        &self.scopes[id]
    }
}

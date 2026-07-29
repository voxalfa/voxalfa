use crate::{
    ast::{
        directives::HeaderDirective,
        params::InitialParams,
        parser::Parser,
        symbols::{Field, FieldAssign, ScopeId},
    },
    data_types::{List, Voice},
    diagnostics::types::DiagnosticKind,
    ts_utils::types::AssignmentData,
};

#[derive(Debug, Default)]
pub struct Header {
    pub sid: ScopeId,
    pub metadata: HeaderMetadata,
    pub params: InitialParams,
    pub directives: HeaderDirective,
}

impl Header {
    pub fn new(scope_id: ScopeId) -> Self {
        Self {
            sid: scope_id,
            ..Default::default()
        }
    }
}

#[derive(Debug, Default)]
pub struct HeaderMetadata {
    pub title: Field<String>,
    pub author: Field<List<String>>,
    pub composer: Field<List<String>>,
    pub voices: Field<List<Voice>>,
    pub verses: Field<usize>,
    pub meter: Field<List<usize>>,
    pub description: Field<String>,
    pub release: Field<usize>,
    pub language: Field<String>,
    pub tags: Field<List<String>>,
}

impl FieldAssign for HeaderMetadata {
    fn assign_field(&mut self, data: AssignmentData, context: &mut Parser) {
        match data.key_name.as_str() {
            "title" => context.assign_field(data, &mut self.title),
            "author" => context.assign_field(data, &mut self.author),
            "composer" => context.assign_field(data, &mut self.composer),
            "voices" => context.assign_field(data, &mut self.voices),
            "verses" => context.assign_field(data, &mut self.verses),
            "meter" => context.assign_field(data, &mut self.meter),
            "description" => context.assign_field(data, &mut self.description),
            "release" => context.assign_field(data, &mut self.release),
            "language" => context.assign_field(data, &mut self.language),
            "tags" => context.assign_field(data, &mut self.tags),
            _ => {
                context.reporter.error(
                    data.full_range,
                    DiagnosticKind::UnknownField(data.key_name.clone()),
                );
            }
        }
    }
}

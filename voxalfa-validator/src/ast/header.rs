use crate::{
    ast::{
        params::CompositionParams,
        symbols::{Field, FieldAssign, ScopeId},
    },
    diagnostic::DiagnosticKind,
    ts_utils::types::AssignmentData,
    validator::DocumentValidator,
};

#[derive(Debug, Default)]
pub struct Header {
    pub sid: ScopeId,
    pub metadata: HeaderMetadata,
    pub params: CompositionParams,
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
    pub author: Field<Vec<String>>,
    pub composer: Field<Vec<String>>,
    pub release: Field<usize>,
    pub description: Field<String>,
}

impl FieldAssign for HeaderMetadata {
    fn assign_field(&mut self, data: AssignmentData, context: &mut DocumentValidator) {
        match data.key_name.as_str() {
            "title" => {
                context.assign_field(data, &mut self.title);
            }
            "author" => {
                context.assign_field(data, &mut self.author);
            }
            "composer" => {
                context.assign_field(data, &mut self.composer);
            }
            "release" => {
                context.assign_field(data, &mut self.release);
            }
            "description" => {
                context.assign_field(data, &mut self.description);
            }
            _ => {
                context.report_error(
                    data.full_range,
                    DiagnosticKind::UnknownField(data.key_name.clone()),
                );
            }
        }
    }
}

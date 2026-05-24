use crate::ast::symbols::{Field, ScopeId};

#[derive(Debug, Default)]
pub struct Header {
    pub sid: ScopeId,
    pub metadata: HeaderMetadata,
    // pub params: CompositionParams,
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

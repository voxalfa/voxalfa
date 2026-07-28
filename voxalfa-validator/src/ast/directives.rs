use crate::{
    ast::{
        parser::Parser,
        symbols::{Field, FieldAssign},
    },
    ts_utils::types::AssignmentData,
};

#[derive(Debug, Default)]
pub struct DirectiveMap {
    pub version: Field<String>,
}

impl FieldAssign for DirectiveMap {
    fn assign_field(&mut self, data: AssignmentData, context: &mut Parser) {
        match data.key_name.as_str() {
            "version" => context.assign_field(data, &mut self.version),
            _ => {
                // TODO: more directives?
            }
        }
    }
}

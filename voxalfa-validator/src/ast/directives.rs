use crate::{
    ast::{
        parser::Parser,
        symbols::{Field, FieldAssign},
    },
    diagnostics::types::DiagnosticKind,
    ts_utils::types::AssignmentData,
};

#[derive(Debug, Default)]
pub struct HeaderDirective {
    pub version: Field<String>,
}

impl FieldAssign for HeaderDirective {
    fn assign_field(&mut self, data: AssignmentData, context: &mut Parser) {
        match data.key_name.as_str() {
            "version" => context.assign_field(data, &mut self.version),
            _ => {
                context.reporter.error(
                    data.full_range,
                    DiagnosticKind::UnknownDirective(data.key_name),
                );
            }
        }
    }
}

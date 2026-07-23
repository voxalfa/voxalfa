use crate::{
    ast::{
        parser::Parser,
        symbols::{FieldAssign, SymbolKind, SymbolRef, Value},
        types::{Dynamic, DynamicKind},
    },
    diagnostics::types::DiagnosticKind,
    ts_utils::types::AssignmentData,
};

#[derive(Debug, Default, Clone)]
pub struct Dynamics {
    pub value: Vec<SymbolRef<Dynamic>>,
}

// FIXME: move to parser.rs?
impl FieldAssign for Dynamics {
    fn assign_field(&mut self, data: AssignmentData, context: &mut Parser) {
        let name = data.key_name.clone();

        if let Ok(kind) = DynamicKind::try_from(name.as_str()) {
            let params = context
                .parse_node::<Vec<_>>(data.value_node)
                .unwrap_or_default();

            let expected_params = kind.expected_params();

            if params.len() != expected_params {
                context.reporter.error(
                    data.value_range,
                    DiagnosticKind::InvalidDynamicParams(expected_params),
                );
            } else {
                let _ = context.tree.add_symbol(
                    SymbolKind::Key(data.key_name.clone()),
                    data.key_range,
                    data.scope_id,
                );

                let sid = context.tree.add_symbol(
                    SymbolKind::Value(Value::Builtin),
                    data.value_range,
                    data.scope_id,
                );

                let value = Dynamic {
                    kind,
                    start: params[0],
                    end: *params.get(1).unwrap_or(&params[0]),
                };

                self.value.push(SymbolRef { sid, value });
            }
        } else {
            context.reporter.error(
                data.full_range,
                DiagnosticKind::InvalidDynamic(name.clone()),
            );
        }
    }
}

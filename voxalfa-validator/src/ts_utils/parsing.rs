use tree_sitter::Node;

use crate::{
    ast::{
        parser::Parser,
        solfa::{BaseNote, Note, NoteVariation},
        symbols::{Primitive, ScopeId, ScopeKind, SymbolKind, SymbolRef, Value},
    },
    data_types::{
        Dynamic, ExtendedTempo, Jump, Key, Mark, StaticTempo, TimeSignature, TimedValue, Touch,
        Voice,
    },
    diagnostics::types::DiagnosticKind,
    ts_utils::{generated::node_types, primitives::ToPrimitve},
};

pub trait ParseNode: Sized {
    fn parse_node(context: &mut Parser, node: Node<'_>, _scope_id: ScopeId) -> Option<Self>;
    fn symbol_kind() -> SymbolKind;
}

pub trait ParseBuiltin: TryFrom<String> {
    const KIND: Primitive;
}

impl ParseNode for usize {
    fn parse_node(context: &mut Parser, node: Node<'_>, _scope_id: ScopeId) -> Option<Self> {
        let range = node.range();

        if node.kind_id() == node_types::INTEGER {
            let text = context.resolve_node_string(node)?;
            let parsed = text.parse::<usize>();

            if let Ok(value) = parsed {
                return Some(value);
            }

            context
                .reporter
                .error(range, DiagnosticKind::InvalidType(Primitive::Integer));
        } else {
            context
                .reporter
                .error(range, DiagnosticKind::ExpectedType("integer", node.kind()));
        }

        None
    }

    fn symbol_kind() -> SymbolKind {
        SymbolKind::primitive_value(Primitive::Integer)
    }
}

impl ParseNode for f32 {
    fn parse_node(context: &mut Parser, node: Node<'_>, _scope_id: ScopeId) -> Option<Self> {
        let range = node.range();

        if matches!(node.kind_id(), node_types::INTEGER | node_types::FLOAT) {
            let text = context.resolve_node_string(node)?;
            let parsed = text.parse::<f32>();

            if let Ok(value) = parsed {
                return Some(value);
            }

            context
                .reporter
                .error(range, DiagnosticKind::InvalidType(Primitive::Float));
        } else {
            context
                .reporter
                .error(range, DiagnosticKind::ExpectedType("float", node.kind()));
        }

        None
    }

    fn symbol_kind() -> SymbolKind {
        SymbolKind::primitive_value(Primitive::Float)
    }
}

impl ParseNode for String {
    fn parse_node(context: &mut Parser, node: Node<'_>, _scope_id: ScopeId) -> Option<Self> {
        let range = node.range();

        if matches!(
            node.kind_id(),
            node_types::STRING | node_types::INLINE_STRING
        ) {
            let value_node = node.named_child(0).unwrap_or(node);
            let value = context.resolve_node_string(value_node)?;

            Some(value)
        } else {
            context
                .reporter
                .error(range, DiagnosticKind::ExpectedType("string", node.kind()));
            None
        }
    }

    fn symbol_kind() -> SymbolKind {
        SymbolKind::primitive_value(Primitive::String)
    }
}

impl ParseNode for bool {
    fn parse_node(context: &mut Parser, node: Node<'_>, _scope_id: ScopeId) -> Option<Self> {
        let range = node.range();

        if node.kind_id() == node_types::BOOLEAN {
            let text = context.resolve_node_string(node)?;

            match text.as_str() {
                "true" => Some(true),
                _ => Some(false),
            }
        } else {
            context
                .reporter
                .error(range, DiagnosticKind::ExpectedType("boolean", node.kind()));
            None
        }
    }

    fn symbol_kind() -> SymbolKind {
        SymbolKind::primitive_value(Primitive::Boolean)
    }
}

impl ParseNode for TimeSignature {
    fn parse_node(context: &mut Parser, node: Node<'_>, scope_id: ScopeId) -> Option<Self> {
        let value = context.parse_node::<Vec<SymbolRef<usize>>>(node, scope_id)?;

        if let [top, bottom] = value.as_slice()
            && top.value + bottom.value > 1
        {
            Some(TimeSignature {
                top: top.value as u8,
                bottom: bottom.value as u8,
            })
        } else {
            context
                .reporter
                .error(node.range(), DiagnosticKind::InvalidTimeSignature);
            None
        }
    }

    fn symbol_kind() -> SymbolKind {
        SymbolKind::Value(Value::List(Primitive::Integer))
    }
}

impl<T> ParseNode for Vec<SymbolRef<T>>
where
    T: ParseNode + ToPrimitve,
{
    fn parse_node(context: &mut Parser, node: Node<'_>, parent_sid: ScopeId) -> Option<Self> {
        if node.kind_id() == node_types::LIST {
            let scope_id = context
                .tree
                .add_scope(ScopeKind::List, node.range(), Some(parent_sid));

            let mut result = Vec::new();

            for child in node.named_children(&mut node.walk()) {
                if let Some(value) = context.parse_node::<T>(child, parent_sid) {
                    let sid = context
                        .tree
                        .add_symbol(T::symbol_kind(), child.range(), scope_id);

                    result.push(SymbolRef { sid, value });
                }
            }

            Some(result)
        } else {
            let value = context.parse_node::<T>(node, parent_sid)?;
            let sid = context
                .tree
                .add_symbol(T::symbol_kind(), node.range(), parent_sid);

            Some(vec![SymbolRef { sid, value }])
        }
    }

    fn symbol_kind() -> SymbolKind {
        SymbolKind::Value(Value::List(T::to_primitive()))
    }
}

impl ParseNode for Note {
    fn parse_node(context: &mut Parser, node: Node<'_>, _scope_id: ScopeId) -> Option<Self> {
        let base_node = node.child_by_field_name("base")?;
        let variation_node = node.child_by_field_name("variation");
        let octave_node = node.child_by_field_name("octave");

        let base_str = context.resolve_node_string(base_node)?;
        let base = BaseNote::try_from(base_str.as_str()).ok()?;

        let variation = variation_node
            .and_then(|v| context.resolve_node_string(v))
            .and_then(|v| NoteVariation::try_from(v.as_str()).ok())
            .unwrap_or_default();

        let octave = octave_node
            .and_then(|n| context.resolve_node_string(n))
            .and_then(|n| n.replace("+", "").parse::<i8>().ok())
            .unwrap_or_default();

        Some(Note {
            base,
            variation,
            octave,
        })
    }

    fn symbol_kind() -> SymbolKind {
        SymbolKind::Token
    }
}

impl<T: ParseNode> ParseNode for TimedValue<T> {
    fn parse_node(context: &mut Parser, node: Node<'_>, scope_id: ScopeId) -> Option<Self> {
        if node.kind_id() == node_types::TIMED_VALUE {
            let value_node = node.child_by_field_name("value")?;
            let start_node = node.child_by_field_name("start")?;
            let end_node = node.child_by_field_name("end");

            let value = context.parse_node(value_node, scope_id)?;
            let start = context.parse_node(start_node, scope_id)?;
            let end = end_node.and_then(|n| context.parse_node(n, scope_id));

            Some(TimedValue { start, end, value })
        } else {
            let value = context.parse_node(node, scope_id)?;

            Some(TimedValue {
                start: 0.,
                end: None,
                value,
            })
        }
    }

    fn symbol_kind() -> SymbolKind {
        SymbolKind::Token
    }
}

impl ParseBuiltin for Dynamic {
    const KIND: Primitive = Primitive::Dynamic;
}

impl ParseBuiltin for Key {
    const KIND: Primitive = Primitive::Key;
}

impl ParseBuiltin for Voice {
    const KIND: Primitive = Primitive::Voice;
}

impl ParseBuiltin for Jump {
    const KIND: Primitive = Primitive::Jump;
}

impl ParseBuiltin for Mark {
    const KIND: Primitive = Primitive::Mark;
}

impl ParseBuiltin for StaticTempo {
    const KIND: Primitive = Primitive::Tempo;
}

impl ParseBuiltin for ExtendedTempo {
    const KIND: Primitive = Primitive::Tempo;
}

impl ParseBuiltin for Touch {
    const KIND: Primitive = Primitive::Touch;
}

impl<T> ParseNode for T
where
    T: ParseBuiltin,
{
    fn parse_node(context: &mut Parser, node: Node<'_>, _scope_id: ScopeId) -> Option<Self> {
        let range = node.range();
        let text = context.resolve_node_string(node)?;

        if let Ok(res) = T::try_from(text) {
            return Some(res);
        }

        context
            .reporter
            .error(range, DiagnosticKind::InvalidType(Self::KIND));

        None
    }

    fn symbol_kind() -> SymbolKind {
        SymbolKind::primitive_value(Self::KIND)
    }
}

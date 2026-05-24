use tree_sitter::Node;

use crate::{
    ast::{
        solfa::{BaseNote, Note, NoteVariation},
        symbols::{SymbolKind, Value},
        types::{Key, TimeSignature, Voice},
    },
    diagnostic::DiagnosticKind,
    validator::DocumentValidator,
};

pub trait ParseNode: Sized {
    fn parse_node(node: Node<'_>, context: &mut DocumentValidator) -> Option<Self>;
    fn symbol_kind() -> SymbolKind;
}

impl ParseNode for usize {
    fn parse_node(node: Node<'_>, context: &mut DocumentValidator) -> Option<Self> {
        let kind = node.kind();
        let range = node.range();

        if kind == "integer" {
            let text = context.resolve_node_string(node)?;
            let parsed = text.parse::<usize>();

            if let Ok(value) = parsed {
                return Some(value);
            }

            context.report_error(range, DiagnosticKind::InvalidType("integer"));
        } else {
            context.report_error(range, DiagnosticKind::ExpectedType("integer", kind));
        }

        None
    }

    fn symbol_kind() -> SymbolKind {
        SymbolKind::Value(Value::Integer)
    }
}

impl ParseNode for String {
    fn parse_node(node: Node<'_>, context: &mut DocumentValidator) -> Option<Self> {
        let kind = node.kind();
        let range = node.range();

        if kind == "string" {
            let value_node = node.named_child(0)?;
            let value = context.resolve_node_string(value_node)?;

            Some(value)
        } else {
            context.report_error(range, DiagnosticKind::ExpectedType("string", kind));
            None
        }
    }

    fn symbol_kind() -> SymbolKind {
        SymbolKind::Value(Value::String)
    }
}

impl ParseNode for bool {
    fn parse_node(node: Node<'_>, context: &mut DocumentValidator) -> Option<Self> {
        let kind = node.kind();
        let range = node.range();

        if kind == "boolean" {
            let text = context.resolve_node_string(node)?;

            match text.as_str() {
                "true" => Some(true),
                _ => Some(false),
            }
        } else {
            context.report_error(range, DiagnosticKind::ExpectedType("boolean", kind));
            None
        }
    }

    fn symbol_kind() -> SymbolKind {
        SymbolKind::Value(Value::Boolean)
    }
}

impl ParseNode for Key {
    fn parse_node(node: Node<'_>, context: &mut DocumentValidator) -> Option<Self> {
        let kind = node.kind();
        let range = node.range();

        if kind == "token" {
            let text = context.resolve_node_string(node)?;

            if let Ok(res) = Key::try_from(text.as_str()) {
                return Some(res);
            }

            context.report_error(range, DiagnosticKind::InvalidType("key"));
        } else {
            context.report_error(range, DiagnosticKind::ExpectedType("key", kind));
        }

        None
    }

    fn symbol_kind() -> SymbolKind {
        SymbolKind::Value(Value::Token)
    }
}

impl ParseNode for Voice {
    fn parse_node(node: Node<'_>, context: &mut DocumentValidator) -> Option<Self> {
        let kind = node.kind();
        let range = node.range();

        if kind == "token" {
            let text = context.resolve_node_string(node)?;

            if let Ok(res) = Voice::try_from(text.as_str()) {
                return Some(res);
            }

            context.report_error(range, DiagnosticKind::InvalidType("voice"));
        } else {
            context.report_error(range, DiagnosticKind::ExpectedType("voice", kind));
        }

        None
    }

    fn symbol_kind() -> SymbolKind {
        SymbolKind::Value(Value::Token)
    }
}

impl ParseNode for TimeSignature {
    fn parse_node(node: Node<'_>, context: &mut DocumentValidator) -> Option<Self> {
        let value = context.parse_node::<Vec<_>>(node)?;

        if value.len() != 2 {
            context.report_error(node.range(), DiagnosticKind::InvalidTimeSignature);
            None
        } else {
            Some(TimeSignature {
                top: value[0],
                bottom: value[1],
            })
        }
    }

    fn symbol_kind() -> SymbolKind {
        SymbolKind::Value(Value::List)
    }
}

impl<T: ParseNode> ParseNode for Vec<T> {
    fn parse_node(node: Node<'_>, context: &mut DocumentValidator) -> Option<Self> {
        let kind = node.kind();

        if kind == "list" {
            let mut result = Vec::new();

            for child in node.named_children(&mut node.walk()) {
                if let Some(value) = context.parse_node(child) {
                    result.push(value);
                }
            }

            Some(result)
        } else {
            context.parse_node(node).map(|v| vec![v])
        }
    }

    fn symbol_kind() -> SymbolKind {
        SymbolKind::Value(Value::List)
    }
}

impl ParseNode for Note {
    fn parse_node(node: Node<'_>, context: &mut DocumentValidator) -> Option<Self> {
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
        SymbolKind::Value(Value::Token)
    }
}

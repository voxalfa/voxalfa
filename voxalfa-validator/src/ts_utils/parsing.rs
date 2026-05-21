use tree_sitter::Node;

use crate::{
    ast::types::{Key, TimeSignature, Voice},
    diagnostic::DiagnosticKind,
    validator::DocumentValidator,
};

pub trait ParseNode: Sized {
    fn parse_node(node: Node<'_>, context: &mut DocumentValidator) -> Option<Self>;
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
}

impl ParseNode for TimeSignature {
    fn parse_node(node: Node<'_>, context: &mut DocumentValidator) -> Option<Self> {
        let value: Vec<usize> = ParseNode::parse_node(node, context)?;

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
}

impl<T: ParseNode> ParseNode for Vec<T> {
    fn parse_node(node: Node<'_>, context: &mut DocumentValidator) -> Option<Self> {
        let kind = node.kind();

        if kind == "list" {
            let mut result = Vec::new();

            for child in node.named_children(&mut node.walk()) {
                if let Some(value) = T::parse_node(child, context) {
                    result.push(value);
                }
            }

            Some(result)
        } else {
            T::parse_node(node, context).map(|v| vec![v])
        }
    }
}

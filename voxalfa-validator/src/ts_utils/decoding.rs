use tree_sitter::Node;

use crate::{
    ast::{
        symbols::{CompositionParams, HeaderMetadata, SectionParams},
        types::{Key, TimeSignature, Voice},
    },
    diagnostic::DiagnosticKind,
    ts_utils::types::AssignmentDataSource,
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

pub trait FieldAssign {
    fn assign_field(&mut self, source: AssignmentDataSource, context: &mut DocumentValidator);
}

impl FieldAssign for HeaderMetadata {
    fn assign_field(&mut self, source: AssignmentDataSource, context: &mut DocumentValidator) {
        match source.data.key.name.as_str() {
            "title" => context.assign_field(source, &mut self.title),
            "author" => context.assign_field(source, &mut self.author),
            "composer" => context.assign_field(source, &mut self.composer),
            "release" => context.assign_field(source, &mut self.release),
            "description" => context.assign_field(source, &mut self.description),
            _ => {}
        }
    }
}

impl FieldAssign for CompositionParams {
    fn assign_field(&mut self, source: AssignmentDataSource, context: &mut DocumentValidator) {
        match source.data.key.name.as_str() {
            "key" => context.assign_field(source, &mut self.key),
            "time" => context.assign_field(source, &mut self.time),
            "bpm" => context.assign_field(source, &mut self.bpm),
            "voices" => context.assign_field(source, &mut self.voices),
            _ => {}
        }
    }
}

impl FieldAssign for SectionParams {
    fn assign_field(&mut self, source: AssignmentDataSource, context: &mut DocumentValidator) {
        match source.data.key.name.as_str() {
            "repeat" => context.assign_field(source, &mut self.repeat),
            _ => self.base.assign_field(source, context),
        }
    }
}

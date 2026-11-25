use thiserror::Error;
use tree_sitter::{Node, Range};

use crate::{
    ast::{
        parser::Parser,
        symbols::{ScopeId, SymbolRef},
    },
    diagnostics::types::DiagnosticKind,
    ts_utils::node_types,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeaderField {
    Title,
    Author,
    Composer,
    Key,
    Time,
    Tempo,
    Meter,
    Language,
    Description,
}

impl HeaderField {
    fn parse(name: &str) -> Option<Self> {
        match name {
            "title" => Some(Self::Title),
            "author" => Some(Self::Author),
            "composer" => Some(Self::Composer),
            "key" => Some(Self::Key),
            "time" => Some(Self::Time),
            "tempo" => Some(Self::Tempo),
            "meter" => Some(Self::Meter),
            "language" => Some(Self::Language),
            "description" => Some(Self::Description),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TemplatePart {
    Text(String),
    Field(HeaderField),
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HeaderTemplate {
    pub parts: Vec<TemplatePart>,
}

impl HeaderTemplate {
    fn parse(source: &str) -> Result<Self, TemplateError> {
        let mut parts = Vec::new();
        let mut text = String::new();
        let mut rest = source;

        while let Some(start) = rest.find("{{") {
            text.push_str(&rest[..start]);
            rest = &rest[start + 2..];

            let Some(end) = rest.find("}}") else {
                return Err(TemplateError::Unclosed);
            };

            let name = rest[..end].trim();

            if name.is_empty() {
                return Err(TemplateError::Empty);
            }

            let field =
                HeaderField::parse(name).ok_or_else(|| TemplateError::Unknown(name.to_string()))?;

            if !text.is_empty() {
                parts.push(TemplatePart::Text(std::mem::take(&mut text)));
            }

            parts.push(TemplatePart::Field(field));
            rest = &rest[end + 2..];
        }

        text.push_str(rest);

        if !text.is_empty() {
            parts.push(TemplatePart::Text(text));
        }

        Ok(Self { parts })
    }
}

#[derive(Debug, Clone, Default)]
pub struct HeaderCell {
    pub templates: Vec<HeaderTemplate>,
    pub styles: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct HeaderRow {
    pub left: Option<HeaderCell>,
    pub center: Option<HeaderCell>,
    pub right: Option<HeaderCell>,
    pub style: Option<String>,
}

impl HeaderRow {
    pub fn is_empty(&self) -> bool {
        self.left.is_none() && self.center.is_none() && self.right.is_none()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
enum TemplateError {
    #[error("unclosed header field")]
    Unclosed,
    #[error("empty header field")]
    Empty,
    #[error("unknown header field '{0}'")]
    Unknown(String),
}

pub fn parse_header_row(
    context: &mut Parser,
    node: Node<'_>,
    scope_id: ScopeId,
) -> Option<HeaderRow> {
    if node.kind_id() != node_types::STYLE_ASSIGNMENTS {
        context.reporter.error(
            node.range(),
            DiagnosticKind::InvalidDirective(
                "header-row".to_string(),
                "expected assignments".to_string(),
            ),
        );

        return None;
    }

    let mut raw = RawRow::default();
    let mut cursor = node.walk();

    for assignment in node.named_children(&mut cursor) {
        let Some((key, key_range, value_node)) = assignment_parts(context, assignment) else {
            continue;
        };

        if key == "style" {
            raw.style = context.parse_node::<String>(value_node, scope_id);
            continue;
        }

        let (name, styles) = match key.strip_suffix("-style") {
            Some(name) => (name, true),
            None => (key.as_str(), false),
        };

        let Some(cell) = raw.cell_mut(name) else {
            context.reporter.error(
                key_range,
                DiagnosticKind::InvalidDirective(
                    "header-row".to_string(),
                    format!("unknown cell '{key}'"),
                ),
            );
            continue;
        };

        if styles {
            cell.styles = parse_styles(context, value_node, scope_id);
        } else {
            cell.templates = parse_templates(context, value_node, scope_id);
        }
    }

    let row = HeaderRow {
        left: raw.left.into_cell(),
        center: raw.center.into_cell(),
        right: raw.right.into_cell(),
        style: raw.style,
    };

    if row.is_empty() {
        context.reporter.error(
            node.range(),
            DiagnosticKind::InvalidDirective(
                "header-row".to_string(),
                "expected at least one of left, center, or right".to_string(),
            ),
        );

        return None;
    }

    Some(row)
}

fn assignment_parts<'a>(context: &mut Parser, node: Node<'a>) -> Option<(String, Range, Node<'a>)> {
    if node.kind_id() != node_types::PARAMETER_ASSIGNMENT {
        return None;
    }

    let key_node = node.child_by_field_name("name")?;
    let value_node = node.child_by_field_name("value")?;
    let key = context.resolve_node_string(key_node)?;

    Some((key, key_node.range(), value_node))
}

fn parse_templates(context: &mut Parser, node: Node<'_>, scope_id: ScopeId) -> Vec<HeaderTemplate> {
    let Some(values) = context.parse_node::<Vec<SymbolRef<String>>>(node, scope_id) else {
        return Vec::new();
    };

    values
        .into_iter()
        .filter_map(|value| {
            let range = context.tree.get_symbol_range(value.sid);

            HeaderTemplate::parse(&value.value)
                .inspect_err(|error| {
                    context.reporter.error(
                        range,
                        DiagnosticKind::InvalidDirective(
                            "header-row".to_string(),
                            error.to_string(),
                        ),
                    );
                })
                .ok()
        })
        .collect()
}

fn parse_styles(context: &mut Parser, node: Node<'_>, scope_id: ScopeId) -> Vec<String> {
    context
        .parse_node::<Vec<SymbolRef<String>>>(node, scope_id)
        .unwrap_or_default()
        .into_iter()
        .map(|value| value.value)
        .collect()
}

#[derive(Default)]
struct RawRow {
    left: RawCell,
    center: RawCell,
    right: RawCell,
    style: Option<String>,
}

impl RawRow {
    fn cell_mut(&mut self, name: &str) -> Option<&mut RawCell> {
        match name {
            "left" => Some(&mut self.left),
            "center" => Some(&mut self.center),
            "right" => Some(&mut self.right),
            _ => None,
        }
    }
}

#[derive(Default)]
struct RawCell {
    templates: Vec<HeaderTemplate>,
    styles: Vec<String>,
}

impl RawCell {
    fn into_cell(self) -> Option<HeaderCell> {
        if self.templates.is_empty() {
            return None;
        }

        Some(HeaderCell {
            templates: self.templates,
            styles: self.styles,
        })
    }
}

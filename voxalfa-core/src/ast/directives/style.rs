use tree_sitter::Node;

use crate::{
    ast::{parser::Parser, symbols::ScopeId},
    diagnostics::types::DiagnosticKind,
    ts_utils::generated::node_types,
};

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct EdgeValues {
    pub top: Option<f32>,
    pub right: Option<f32>,
    pub bottom: Option<f32>,
    pub left: Option<f32>,
}

impl EdgeValues {
    fn all(value: f32) -> Self {
        Self {
            top: Some(value),
            right: Some(value),
            bottom: Some(value),
            left: Some(value),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ElementStyle {
    pub font_family: Option<String>,
    pub font_size: Option<f32>,
    pub margin: EdgeValues,
    pub padding: EdgeValues,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct PageStyle {
    pub width: Option<f32>,
    pub height: Option<f32>,
    pub margin: EdgeValues,
}

struct StyleAssignment<'a> {
    key: String,
    key_node: Node<'a>,
    value_node: Node<'a>,
}

pub(super) fn parse_page_style(
    context: &mut Parser,
    node: Node<'_>,
    scope_id: ScopeId,
    name: &str,
) -> Option<PageStyle> {
    if node.kind_id() != node_types::STYLE_ASSIGNMENTS {
        invalid_directive(context, node.range(), name, "expected style assignments");
        return None;
    }

    let mut style = PageStyle::default();
    let mut cursor = node.walk();

    for assignment in node.named_children(&mut cursor) {
        if assignment.kind_id() != node_types::PARAMETER_ASSIGNMENT {
            continue;
        }

        let assignment = assignment_parts(context, assignment)?;

        match assignment.key.as_str() {
            "width" => {
                let width = number_value(context, assignment.value_node, scope_id, name)?;

                if !width.is_finite() || width <= 0.0 {
                    invalid_directive(
                        context,
                        assignment.key_node.range(),
                        name,
                        "width must be positive",
                    );
                    return None;
                }

                style.width = Some(width);
            }
            "height" => {
                let height = number_value(context, assignment.value_node, scope_id, name)?;

                if !height.is_finite() || height <= 0.0 {
                    invalid_directive(
                        context,
                        assignment.key_node.range(),
                        name,
                        "height must be positive",
                    );
                    return None;
                }

                style.height = Some(height);
            }
            _ => apply_margin_modifier(context, name, &assignment, scope_id, &mut style.margin)?,
        }
    }

    Some(style)
}

pub(super) fn parse_element_style(
    context: &mut Parser,
    node: Node<'_>,
    scope_id: ScopeId,
    name: &str,
) -> Option<ElementStyle> {
    if node.kind_id() != node_types::STYLE_ASSIGNMENTS {
        invalid_directive(context, node.range(), name, "expected style assignments");
        return None;
    }

    let mut style = ElementStyle::default();
    let mut cursor = node.walk();

    for assignment in node.named_children(&mut cursor) {
        if assignment.kind_id() != node_types::PARAMETER_ASSIGNMENT {
            continue;
        }

        let assignment = assignment_parts(context, assignment)?;

        match assignment.key.as_str() {
            "font-family" | "font-familly" => {
                let family = string_value(context, assignment.value_node, scope_id)?;

                if family.is_empty() {
                    invalid_directive(
                        context,
                        assignment.key_node.range(),
                        name,
                        "font-family must not be empty",
                    );
                    return None;
                }

                style.font_family = Some(family);
            }
            "font-size" => {
                let size = number_value(context, assignment.value_node, scope_id, name)?;

                if !size.is_finite() || size <= 0.0 {
                    invalid_directive(
                        context,
                        assignment.key_node.range(),
                        name,
                        "font-size must be positive",
                    );
                    return None;
                }

                style.font_size = Some(size);
            }
            _ => apply_box_modifier(
                context,
                name,
                &assignment,
                scope_id,
                &mut style.margin,
                &mut style.padding,
            )?,
        }
    }

    Some(style)
}

fn assignment_parts<'a>(context: &mut Parser, node: Node<'a>) -> Option<StyleAssignment<'a>> {
    let key_node = node.child_by_field_name("name")?;
    let value_node = node.child_by_field_name("value")?;
    let key = context.resolve_node_string(key_node)?;

    Some(StyleAssignment {
        key,
        key_node,
        value_node,
    })
}

fn apply_box_modifier(
    context: &mut Parser,
    name: &str,
    assignment: &StyleAssignment<'_>,
    scope_id: ScopeId,
    margin: &mut EdgeValues,
    padding: &mut EdgeValues,
) -> Option<()> {
    let value = number_value(context, assignment.value_node, scope_id, name)?;
    let (kind, side) = split_key(&assignment.key);

    let edges = match kind {
        "margin" => margin,
        "padding" => padding,
        _ => {
            invalid_directive(context, assignment.key_node.range(), name, &assignment.key);
            return None;
        }
    };

    apply_edges(context, name, assignment, side, edges, value)
}

fn apply_margin_modifier(
    context: &mut Parser,
    name: &str,
    assignment: &StyleAssignment<'_>,
    scope_id: ScopeId,
    margin: &mut EdgeValues,
) -> Option<()> {
    let value = number_value(context, assignment.value_node, scope_id, name)?.max(0.0);
    let (kind, side) = split_key(&assignment.key);

    if kind != "margin" {
        invalid_directive(context, assignment.key_node.range(), name, &assignment.key);
        return None;
    }

    apply_edges(context, name, assignment, side, margin, value)
}

fn apply_edges(
    context: &mut Parser,
    name: &str,
    assignment: &StyleAssignment<'_>,
    side: Option<&str>,
    edges: &mut EdgeValues,
    value: f32,
) -> Option<()> {
    match side {
        None => *edges = EdgeValues::all(value),
        Some("top") => edges.top = Some(value),
        Some("right") => edges.right = Some(value),
        Some("bottom") => edges.bottom = Some(value),
        Some("left") => edges.left = Some(value),
        Some("x") => {
            edges.left = Some(value);
            edges.right = Some(value);
        }
        Some("y") => {
            edges.top = Some(value);
            edges.bottom = Some(value);
        }
        _ => {
            invalid_directive(context, assignment.key_node.range(), name, &assignment.key);
            return None;
        }
    }

    Some(())
}

fn number_value(
    context: &mut Parser,
    node: Node<'_>,
    scope_id: ScopeId,
    name: &str,
) -> Option<f32> {
    if matches!(node.kind_id(), node_types::INTEGER | node_types::FLOAT) {
        context.parse_node::<f32>(node, scope_id)
    } else {
        invalid_directive(context, node.range(), name, "expected a number");
        None
    }
}

fn string_value(context: &mut Parser, node: Node<'_>, scope_id: ScopeId) -> Option<String> {
    match node.kind_id() {
        node_types::STRING | node_types::INLINE_STRING => {
            context.parse_node::<String>(node, scope_id)
        }
        _ => context.resolve_node_string(node),
    }
}

fn invalid_directive(
    context: &mut Parser,
    range: crate::ts_utils::range::Range,
    name: &str,
    message: &str,
) {
    context.reporter.error(
        range,
        DiagnosticKind::InvalidDirective(name.to_string(), message.to_string()),
    );
}

fn split_key(key: &str) -> (&str, Option<&str>) {
    match key.split_once('-') {
        Some((kind, side)) => (kind, Some(side)),
        None => (key, None),
    }
}

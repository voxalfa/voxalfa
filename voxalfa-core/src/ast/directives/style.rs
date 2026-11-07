use tree_sitter::Node;

use crate::{
    ast::{parser::Parser, symbols::ScopeId},
    diagnostics::types::DiagnosticKind,
    ts_utils::node_types,
};

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct EdgeValues {
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub left: f32,
}

impl EdgeValues {
    pub fn all(value: f32) -> Self {
        Self {
            top: value,
            right: value,
            bottom: value,
            left: value,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub enum FontStretch {
    UltraCondensed,
    ExtraCondensed,
    Condensed,
    SemiCondensed,
    #[default]
    Normal,
    SemiExpanded,
    Expanded,
    ExtraExpanded,
    UltraExpanded,
}

impl FontStretch {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "ultra-condensed" => Some(Self::UltraCondensed),
            "extra-condensed" => Some(Self::ExtraCondensed),
            "condensed" => Some(Self::Condensed),
            "semi-condensed" => Some(Self::SemiCondensed),
            "normal" => Some(Self::Normal),
            "semi-expanded" => Some(Self::SemiExpanded),
            "expanded" => Some(Self::Expanded),
            "extra-expanded" => Some(Self::ExtraExpanded),
            "ultra-expanded" => Some(Self::UltraExpanded),
            _ => None,
        }
    }

    pub fn rank(self) -> u8 {
        match self {
            Self::UltraCondensed => 0,
            Self::ExtraCondensed => 1,
            Self::Condensed => 2,
            Self::SemiCondensed => 3,
            Self::Normal => 4,
            Self::SemiExpanded => 5,
            Self::Expanded => 6,
            Self::ExtraExpanded => 7,
            Self::UltraExpanded => 8,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ElementStyle {
    pub font_family: Option<String>,
    pub font_size: Option<f32>,
    pub font_weight: Option<u16>,
    pub font_stretch: Option<FontStretch>,
    pub italic: Option<bool>,
    pub color: Option<String>,
    pub margin: EdgeValues,
    pub padding: EdgeValues,
}

impl ElementStyle {
    pub fn top(&self) -> f32 {
        self.margin.top + self.padding.top
    }

    pub fn right(&self) -> f32 {
        self.margin.right + self.padding.right
    }

    pub fn bottom(&self) -> f32 {
        self.margin.bottom + self.padding.bottom
    }

    pub fn left(&self) -> f32 {
        self.margin.left + self.padding.left
    }

    pub fn size(&self, default: f32) -> f32 {
        self.font_size.unwrap_or(default)
    }
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

pub fn parse_page_style(
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
                let width = context.parse_node::<f32>(assignment.value_node, scope_id)?;

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
                let height = context.parse_node::<f32>(assignment.value_node, scope_id)?;

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

pub fn parse_element_style(
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
        let key_node = assignment.key_node;
        let value_node = assignment.value_node;

        match assignment.key.as_str() {
            "font-family" => {
                let family = context.parse_node::<String>(value_node, scope_id)?;

                style.font_family = Some(family);
            }
            "font-style" => {
                let value = context.parse_node::<String>(value_node, scope_id)?;

                match value.as_str() {
                    "normal" => style.italic = Some(false),
                    "italic" => style.italic = Some(true),
                    _ => invalid_directive(context, key_node.range(), name, "invalid style"),
                }
            }
            "font-weight" => {
                let value = context.parse_node::<String>(value_node, scope_id)?;

                if let Some(weight) = font_weight(&value) {
                    style.font_weight = Some(weight);
                } else {
                    invalid_directive(context, key_node.range(), name, "invalid weight value");
                }
            }
            "font-stretch" => {
                let value = context.parse_node::<String>(value_node, scope_id)?;

                if let Some(stretch) = FontStretch::parse(&value) {
                    style.font_stretch = Some(stretch);
                } else {
                    invalid_directive(context, key_node.range(), name, "invalid stretch value");
                }
            }
            "font-size" => {
                let size = context.parse_node::<f32>(value_node, scope_id)?;

                if !size.is_finite() || size <= 0.0 {
                    invalid_directive(context, key_node.range(), name, "non-positive value");
                } else {
                    style.font_size = Some(size);
                }
            }
            "color" => {
                let color = context.parse_node::<String>(value_node, scope_id)?;

                style.color = Some(color);
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
    let value = context.parse_node::<f32>(assignment.value_node, scope_id)?;
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
    let value = context.parse_node::<f32>(assignment.value_node, scope_id)?;
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
        Some("top") => edges.top = value,
        Some("right") => edges.right = value,
        Some("bottom") => edges.bottom = value,
        Some("left") => edges.left = value,
        Some("x") => {
            edges.left = value;
            edges.right = value;
        }
        Some("y") => {
            edges.top = value;
            edges.bottom = value;
        }
        _ => {
            invalid_directive(context, assignment.key_node.range(), name, &assignment.key);
            return None;
        }
    }

    Some(())
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

fn font_weight(weight: &str) -> Option<u16> {
    match weight {
        "thin" => Some(100),
        "extra-light" => Some(200),
        "light" => Some(300),
        "normal" => Some(400),
        "medium" => Some(500),
        "semibold" => Some(600),
        "bold" => Some(700),
        "extra-bold" => Some(800),
        "black" => Some(900),
        _ => None,
    }
}

mod style;

pub use style::{EdgeValues, ElementStyle, PageStyle};

use std::collections::BTreeMap;

use crate::{
    ast::{
        parser::Parser,
        symbols::{Field, FieldAssign, SymbolKind, SymbolRef},
    },
    diagnostics::types::DiagnosticKind,
    ts_utils::types::AssignmentData,
};

use style::{parse_element_style, parse_page_style};

pub const STYLE_ELEMENTS: &[&str] = &[
    "title-text",
    "author-text",
    "composer-text",
    "key-text",
    "time-text",
    "meter-text",
    "note-text",
    "octave-text",
    "lyrics-text",
    "divider-text",
    "key-label",
    "time-label",
    "tempo-label",
    "jump-label",
    "mark-label",
    "dynamic-label",
    "volta-text",
    "page-number-text",
];

#[derive(Debug, Default)]
pub struct HeaderDirective {
    pub version: Field<String>,
    pub page: Field<PageStyle>,
    pub elements: BTreeMap<String, SymbolRef<ElementStyle>>,
}

impl FieldAssign for HeaderDirective {
    fn assign_field(&mut self, data: AssignmentData, context: &mut Parser) {
        match data.key_name.as_str() {
            "version" => context.assign_field(data, &mut self.version),
            "page" => self.assign_page(data, context),
            name if STYLE_ELEMENTS.contains(&name) => self.assign_element(data, context),
            _ => {
                context.reporter.error(
                    data.full_range,
                    DiagnosticKind::UnknownDirective(data.key_name),
                );
            }
        }
    }
}

impl HeaderDirective {
    fn assign_page(&mut self, data: AssignmentData, context: &mut Parser) {
        let Some(style) = parse_page_style(context, data.value_node, data.scope_id, "page") else {
            return;
        };

        let sid = context
            .tree
            .add_symbol(SymbolKind::Token, data.value_range, data.scope_id);

        self.page = Some(SymbolRef { sid, value: style });
    }

    fn assign_element(&mut self, data: AssignmentData, context: &mut Parser) {
        let Some(style) =
            parse_element_style(context, data.value_node, data.scope_id, &data.key_name)
        else {
            return;
        };

        let sid = context
            .tree
            .add_symbol(SymbolKind::Token, data.value_range, data.scope_id);

        self.elements
            .insert(data.key_name, SymbolRef { sid, value: style });
    }
}

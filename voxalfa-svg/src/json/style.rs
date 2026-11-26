use std::collections::BTreeMap;

use serde::Serialize;
use voxalfa_core::{
    ast::directives::{EdgeValues, FontStretch, STYLE_ELEMENTS},
    output::FinalOutput,
};

use crate::{
    RenderOptions,
    fonts::{TEXT_FAMILY, default_font_face},
};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HeaderStyle {
    pub page: PageStyle,
    pub header: BlockStyle,
    pub system: BlockStyle,
    pub styles: BTreeMap<String, ElementStyle>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PageStyle {
    pub width: f32,
    pub height: f32,
    pub margin: Edges,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BlockStyle {
    pub margin: Edges,
    pub padding: Edges,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Edges {
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub left: f32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ElementStyle {
    pub font_family: String,
    pub font_size: f32,
    pub font_weight: u16,
    pub font_stretch: String,
    pub font_style: String,
    pub color: Option<String>,
    pub margin: Edges,
    pub padding: Edges,
}

impl HeaderStyle {
    pub fn defaults() -> Self {
        Self::from_options(&RenderOptions::default())
    }

    pub fn from_output(output: &FinalOutput) -> Self {
        let mut options = RenderOptions::default();
        options.apply_directives(&output.header.directives);

        Self::from_options(&options)
    }

    pub fn from_options(options: &RenderOptions) -> Self {
        let mut names = STYLE_ELEMENTS
            .iter()
            .map(|name| name.to_string())
            .collect::<Vec<_>>();

        names.extend(options.elements.custom.keys().cloned());
        names.sort();
        names.dedup();

        let styles = names
            .into_iter()
            .filter_map(|name| {
                let style = resolve_style(&name, options)?;

                Some((name, style))
            })
            .collect();

        Self {
            page: PageStyle {
                width: options.page.width,
                height: options.page.height,
                margin: edges(options.page_margin),
            },
            header: BlockStyle {
                margin: edges(options.header_layout.margin),
                padding: edges(options.header_layout.padding),
            },
            system: BlockStyle {
                margin: edges(options.system_layout.margin),
                padding: edges(options.system_layout.padding),
            },
            styles,
        }
    }
}

fn resolve_style(name: &str, options: &RenderOptions) -> Option<ElementStyle> {
    let style = options.elements.get(name)?;
    let (family, weight, italic, stretch) =
        default_font_face(name).unwrap_or((TEXT_FAMILY, 400, false, FontStretch::Normal));

    Some(ElementStyle {
        font_family: style
            .font_family
            .clone()
            .unwrap_or_else(|| family.to_string()),
        font_size: style
            .font_size
            .unwrap_or_else(|| default_font_size(name, options)),
        font_weight: style.font_weight.unwrap_or(weight),
        font_stretch: style
            .font_stretch
            .map(font_stretch_name)
            .unwrap_or_else(|| font_stretch_name(stretch))
            .to_string(),
        font_style: if style.italic.unwrap_or(italic) {
            "italic".to_string()
        } else {
            "normal".to_string()
        },
        color: style.color.clone(),
        margin: edges(style.margin),
        padding: edges(style.padding),
    })
}

fn default_font_size(name: &str, options: &RenderOptions) -> f32 {
    match name {
        "note-text" | "divider-text" => options.solfa_font_size,
        "octave-text" => options.octave_font_size,
        "lyrics-text" => options.lyric_font_size,
        "title-text" => options.title_font_size,
        "author-text" | "composer-text" => options.name_font_size,
        "header-text" | "header-row" | "meter-text" => options.meta_font_size,
        "footer-text" => options.footer_font_size,
        _ => options.marker_font_size,
    }
}

fn edges(value: EdgeValues) -> Edges {
    Edges {
        top: value.top,
        right: value.right,
        bottom: value.bottom,
        left: value.left,
    }
}

fn font_stretch_name(stretch: FontStretch) -> &'static str {
    match stretch {
        FontStretch::UltraCondensed => "ultra-condensed",
        FontStretch::ExtraCondensed => "extra-condensed",
        FontStretch::Condensed => "condensed",
        FontStretch::SemiCondensed => "semi-condensed",
        FontStretch::Normal => "normal",
        FontStretch::SemiExpanded => "semi-expanded",
        FontStretch::Expanded => "expanded",
        FontStretch::ExtraExpanded => "extra-expanded",
        FontStretch::UltraExpanded => "ultra-expanded",
    }
}

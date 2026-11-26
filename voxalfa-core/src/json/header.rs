use std::collections::BTreeMap;

use serde::Serialize;

use crate::{
    ast::{
        directives::{
            BlockStyle, ElementStyle, FontStretch, HeaderCell, HeaderField, HeaderRow,
            HeaderTemplate, PageStyle, TemplatePart,
        },
        header::Header,
    },
    error::Error,
    output::FinalOutput,
    validation::Validator,
};

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct HeaderDump {
    pub metadata: HeaderMetadataDump,
    pub params: HeaderParamsDump,
    pub directives: HeaderDirectivesDump,
}

#[derive(Debug, Clone, Default, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct HeaderMetadataDump {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    pub authors: Vec<String>,
    pub composers: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verses: Option<usize>,
    pub meter: Vec<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub release: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct HeaderParamsDump {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time: Option<TimeSignatureDump>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tempo: Option<String>,
    pub voices: Vec<String>,
}

#[derive(Debug, Clone, Copy, Default, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TimeSignatureDump {
    pub top: u8,
    pub bottom: u8,
}

#[derive(Debug, Clone, Default, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct HeaderDirectivesDump {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page: Option<PageDump>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub header: Option<BlockStyleDump>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<BlockStyleDump>,
    pub styles: BTreeMap<String, StyleDump>,
    pub rows: Vec<HeaderRowDump>,
}

#[derive(Debug, Clone, Default, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PageDump {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<f32>,
    pub margin: EdgesDump,
}

#[derive(Debug, Clone, Default, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BlockStyleDump {
    pub margin: EdgesDump,
    pub padding: EdgesDump,
}

#[derive(Debug, Clone, Default, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct EdgesDump {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub right: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bottom: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub left: Option<f32>,
}

#[derive(Debug, Clone, Default, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct StyleDump {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub font_family: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub font_size: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub font_weight: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub font_stretch: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub font_style: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    pub margin: EdgesDump,
    pub padding: EdgesDump,
}

#[derive(Debug, Clone, Default, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct HeaderRowDump {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub left: Option<HeaderCellDump>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub center: Option<HeaderCellDump>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub right: Option<HeaderCellDump>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct HeaderCellDump {
    pub templates: Vec<String>,
    pub styles: Vec<String>,
}

impl HeaderDump {
    pub fn from_source(source: &str) -> Result<Self, Error> {
        let mut validator = Validator::init()?;
        let output = validator.analyze(source);

        Ok(Self::from_output(&output))
    }

    pub fn from_output(output: &FinalOutput) -> Self {
        Self::from_header(&output.header)
    }

    pub fn from_header(header: &Header) -> Self {
        Self {
            metadata: dump_metadata(header),
            params: dump_params(header),
            directives: dump_directives(header),
        }
    }

    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    pub fn to_json_pretty(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }
}

fn dump_metadata(header: &Header) -> HeaderMetadataDump {
    let metadata = &header.metadata;

    HeaderMetadataDump {
        title: metadata.title.as_ref().map(|value| value.value.clone()),
        authors: metadata
            .author
            .as_ref()
            .map(|list| list.value.iter().map(|item| item.value.clone()).collect())
            .unwrap_or_default(),
        composers: metadata
            .composer
            .as_ref()
            .map(|list| list.value.iter().map(|item| item.value.clone()).collect())
            .unwrap_or_default(),
        verses: metadata.verses.as_ref().map(|value| value.value),
        meter: metadata
            .meter
            .as_ref()
            .map(|list| list.value.iter().map(|item| item.value).collect())
            .unwrap_or_default(),
        description: metadata
            .description
            .as_ref()
            .map(|value| value.value.clone()),
        release: metadata.release.as_ref().map(|value| value.value),
        language: metadata.language.as_ref().map(|value| value.value.clone()),
        tags: metadata
            .tags
            .as_ref()
            .map(|list| list.value.iter().map(|item| item.value.clone()).collect())
            .unwrap_or_default(),
    }
}

fn dump_params(header: &Header) -> HeaderParamsDump {
    let params = &header.params;

    HeaderParamsDump {
        key: params.key.as_ref().map(|value| value.value.to_string()),
        time: params.time.as_ref().map(|value| TimeSignatureDump {
            top: value.value.top,
            bottom: value.value.bottom,
        }),
        tempo: params.tempo.as_ref().map(|value| value.value.to_string()),
        voices: params
            .voices
            .as_ref()
            .map(|list| {
                list.value
                    .iter()
                    .map(|item| item.value.to_string())
                    .collect()
            })
            .unwrap_or_default(),
    }
}

fn dump_directives(header: &Header) -> HeaderDirectivesDump {
    let directives = &header.directives;

    HeaderDirectivesDump {
        version: directives.version.as_ref().map(|value| value.value.clone()),
        page: directives
            .page
            .as_ref()
            .map(|value| dump_page(&value.value)),
        header: directives
            .header
            .as_ref()
            .map(|value| dump_block(&value.value)),
        system: directives
            .system
            .as_ref()
            .map(|value| dump_block(&value.value)),
        styles: directives
            .elements
            .iter()
            .map(|(name, style)| (name.clone(), dump_style(&style.value)))
            .collect(),
        rows: directives.rows.iter().map(dump_row).collect(),
    }
}

fn dump_page(page: &PageStyle) -> PageDump {
    PageDump {
        width: page.width,
        height: page.height,
        margin: dump_edges(
            page.margin.top,
            page.margin.right,
            page.margin.bottom,
            page.margin.left,
        ),
    }
}

fn dump_block(block: &BlockStyle) -> BlockStyleDump {
    BlockStyleDump {
        margin: dump_edges(
            block.margin.top,
            block.margin.right,
            block.margin.bottom,
            block.margin.left,
        ),
        padding: dump_edges(
            block.padding.top,
            block.padding.right,
            block.padding.bottom,
            block.padding.left,
        ),
    }
}

fn dump_style(style: &ElementStyle) -> StyleDump {
    StyleDump {
        font_family: style.font_family.clone(),
        font_size: style.font_size,
        font_weight: style.font_weight,
        font_stretch: style
            .font_stretch
            .map(font_stretch_name)
            .map(str::to_string),
        font_style: style.italic.map(|italic| {
            if italic {
                "italic".to_string()
            } else {
                "normal".to_string()
            }
        }),
        color: style.color.clone(),
        margin: dump_edges(
            style.margin.top,
            style.margin.right,
            style.margin.bottom,
            style.margin.left,
        ),
        padding: dump_edges(
            style.padding.top,
            style.padding.right,
            style.padding.bottom,
            style.padding.left,
        ),
    }
}

fn dump_row(row: &HeaderRow) -> HeaderRowDump {
    HeaderRowDump {
        left: row.left.as_ref().map(dump_cell),
        center: row.center.as_ref().map(dump_cell),
        right: row.right.as_ref().map(dump_cell),
        style: row.style.clone(),
    }
}

fn dump_cell(cell: &HeaderCell) -> HeaderCellDump {
    HeaderCellDump {
        templates: cell.templates.iter().map(dump_template).collect(),
        styles: cell.styles.clone(),
    }
}

fn dump_template(template: &HeaderTemplate) -> String {
    template
        .parts
        .iter()
        .map(|part| match part {
            TemplatePart::Text(text) => text.clone(),
            TemplatePart::Field(field) => format!("{{{{{}}}}}", header_field_name(*field)),
        })
        .collect()
}

fn dump_edges(top: f32, right: f32, bottom: f32, left: f32) -> EdgesDump {
    EdgesDump {
        top: finite(top),
        right: finite(right),
        bottom: finite(bottom),
        left: finite(left),
    }
}

fn finite(value: f32) -> Option<f32> {
    value.is_finite().then_some(value)
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

fn header_field_name(field: HeaderField) -> &'static str {
    match field {
        HeaderField::Title => "title",
        HeaderField::Author => "author",
        HeaderField::Composer => "composer",
        HeaderField::Key => "key",
        HeaderField::Time => "time",
        HeaderField::Tempo => "tempo",
        HeaderField::Meter => "meter",
        HeaderField::Language => "language",
        HeaderField::Description => "description",
    }
}

#[cfg(test)]
mod tests {
    use super::HeaderDump;

    #[test]
    fn test_header_dump() {
        let source = r##"
;; @version 0.1.0-alpha.3
;; @page width={600} | margin={20}
;; @style name="title-text" | font-size={24} | color="#222222"
;; @header-row center="{{title}}" | center-style="title-text"

[#] title="Hello" | author={"Alice", "Bob"} | verses={2}
[#] meter={4,4} | language="en"

[$] key={G} | time={4,4} | tempo={100} | voices={S,A}

---

[S] |d :r !m :f ||
"##;

        let dump = HeaderDump::from_source(source).expect("failed to dump header");
        println!("{}", dump.to_json_pretty().unwrap());

        assert_eq!(dump.metadata.title.as_deref(), Some("Hello"));
        assert_eq!(dump.metadata.authors, vec!["Alice", "Bob"]);
        assert_eq!(dump.metadata.meter, vec![4, 4]);
        assert_eq!(dump.params.key.as_deref(), Some("G"));
        assert_eq!(dump.params.time.map(|time| time.top), Some(4));
        assert_eq!(dump.directives.styles["title-text"].font_size, Some(24.0));
        assert_eq!(
            dump.directives.rows[0].center.as_ref().unwrap().templates[0],
            "{{title}}"
        );
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PageSize {
    pub width: f32,
    pub height: f32,
}

impl PageSize {
    pub const A4: Self = Self {
        width: 794.0,
        height: 1123.0,
    };
}

use std::collections::BTreeMap;

use voxalfa_core::ast::directives::{BlockStyle, EdgeValues, ElementStyle, HeaderDirective};

use crate::renderer::constants::{
    HEADER_KEY_RIGHT_MARGIN, HEADER_ROW_MARGIN_BOTTOM, HEADER_TIME_RIGHT_MARGIN,
};

#[derive(Debug, Clone)]
pub struct RenderOptions {
    pub page: PageSize,

    pub solfa_font_size: f32,
    pub lyric_font_size: f32,
    pub octave_font_size: f32,
    pub title_font_size: f32,
    pub name_font_size: f32,
    pub meta_font_size: f32,
    pub marker_font_size: f32,
    pub footer_font_size: f32,
    pub encode_font: bool,

    pub page_margin: EdgeValues,
    pub col_padding: f32,
    pub min_col_width: f32,
    pub voice_row_height: f32,
    pub lyric_row_height: f32,

    pub system_layout: BlockStyle,
    pub header_layout: BlockStyle,

    pub elements: ElementStyles,
}

fn default_header_layout() -> BlockStyle {
    BlockStyle {
        margin: EdgeValues {
            bottom: 20.0,
            ..EdgeValues::default()
        },
        padding: EdgeValues::default(),
    }
}

fn default_system_layout() -> BlockStyle {
    BlockStyle {
        margin: EdgeValues {
            top: 0.0,
            bottom: 10.0,
            ..EdgeValues::default()
        },
        padding: EdgeValues::default(),
    }
}

fn default_element_styles() -> ElementStyles {
    let mut styles = ElementStyles::default();

    styles.header_row.margin.bottom = HEADER_ROW_MARGIN_BOTTOM;
    styles.key.margin.right = HEADER_KEY_RIGHT_MARGIN;
    styles.time.margin.right = HEADER_TIME_RIGHT_MARGIN;

    styles
}

impl Default for RenderOptions {
    fn default() -> Self {
        Self {
            page: PageSize::A4,
            solfa_font_size: 12.0,
            lyric_font_size: 11.0,
            octave_font_size: 7.0,
            title_font_size: 18.0,
            name_font_size: 13.0,
            meta_font_size: 13.0,
            marker_font_size: 11.0,
            footer_font_size: 11.0,
            encode_font: true,
            page_margin: EdgeValues::all(56.7),
            col_padding: 2.0,
            min_col_width: 12.0,
            voice_row_height: 24.0,
            lyric_row_height: 20.0,
            system_layout: default_system_layout(),
            header_layout: default_header_layout(),
            elements: default_element_styles(),
        }
    }
}

impl RenderOptions {
    pub fn printable_width(&self) -> f32 {
        (self.page.width - self.page_margin.left - self.page_margin.right).max(0.0)
    }

    pub fn printable_height(&self) -> f32 {
        (self.page.height - self.page_margin.top - self.page_margin.bottom).max(0.0)
    }

    pub fn apply_directives(&mut self, directives: &HeaderDirective) {
        if let Some(page) = &directives.page {
            let page = &page.value;

            if let Some(width) = page.width {
                self.page.width = width;
            }

            if let Some(height) = page.height {
                self.page.height = height;
            }

            self.page_margin.merge(&page.margin);
        }

        for (name, style) in &directives.elements {
            self.elements.insert(name, style.value.clone());
        }

        if let Some(header) = &directives.header {
            self.header_layout.margin.merge(&header.value.margin);
            self.header_layout.padding.merge(&header.value.padding);
        }

        if let Some(system) = &directives.system {
            self.system_layout.margin.merge(&system.value.margin);
            self.system_layout.padding.merge(&system.value.padding);
        }
    }
}

macro_rules! define_element_styles {
    ($($name:literal => $field:ident),* $(,)?) => {
        #[derive(Debug, Clone, Default)]
        pub struct ElementStyles {
            $(pub $field: ElementStyle,)*
            pub custom: BTreeMap<String, ElementStyle>,
        }

        impl ElementStyles {
            pub const NAMES: &[&str] = &[$($name),*];

            pub fn get(&self, name: &str) -> Option<&ElementStyle> {
                match name {
                    $($name => Some(&self.$field),)*
                    _ => self.custom.get(name),
                }
            }

            fn insert(&mut self, name: &str, style: ElementStyle) {
                match name {
                    $($name => self.$field.merge(&style),)*
                    _ => {
                        self.custom.entry(name.to_string()).or_default().merge(&style);
                    }
                }
            }
        }
    };
}

define_element_styles! {
    "title-text" => title,
    "author-text" => author,
    "composer-text" => composer,
    "header-text" => header,
    "header-row" => header_row,
    "key-text" => key,
    "time-text" => time,
    "meter-text" => meter,
    "note-text" => note,
    "octave-text" => octave,
    "lyrics-text" => lyrics,
    "divider-text" => divider,
    "footer-text" => footer,
    "key-label" => key_label,
    "time-label" => time_label,
    "tempo-label" => tempo,
    "bpm-label" => bpm,
    "jump-label" => jump,
    "mark-label" => mark,
    "dynamic-label" => dynamic,
    "volta-label" => volta,
}

#[cfg(test)]
mod tests {
    use super::ElementStyles;

    #[test]
    fn test_element_style_integrity() {
        assert_eq!(
            ElementStyles::NAMES,
            voxalfa_core::ast::directives::STYLE_ELEMENTS
        );
    }
}

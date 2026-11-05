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

use voxalfa_core::ast::directives::{
    EdgeValues, ElementStyle as CoreElementStyle, HeaderDirective,
};

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Edges {
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub left: f32,
}

impl Edges {
    fn all(value: f32) -> Self {
        Self {
            top: value,
            right: value,
            bottom: value,
            left: value,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct ElementStyle {
    pub font_family: Option<String>,
    pub font_size: Option<f32>,
    pub margin: Edges,
    pub padding: Edges,
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

#[derive(Debug, Clone, Default)]
pub struct ElementStyles {
    pub text: TextStyles,
    pub label: LabelStyles,
}

#[derive(Debug, Clone, Default)]
pub struct TextStyles {
    pub title: ElementStyle,
    pub author: ElementStyle,
    pub composer: ElementStyle,
    pub key: ElementStyle,
    pub time: ElementStyle,
    pub meter: ElementStyle,
    pub note: ElementStyle,
    pub octave: ElementStyle,
    pub lyrics: ElementStyle,
    pub divider: ElementStyle,
    pub volta: ElementStyle,
    pub page_number: ElementStyle,
}

#[derive(Debug, Clone, Default)]
pub struct LabelStyles {
    pub key: ElementStyle,
    pub time: ElementStyle,
    pub tempo: ElementStyle,
    pub jump: ElementStyle,
    pub mark: ElementStyle,
    pub dynamic: ElementStyle,
}

impl ElementStyles {
    fn get_mut(&mut self, name: &str) -> Option<&mut ElementStyle> {
        match name {
            "title-text" => Some(&mut self.text.title),
            "author-text" => Some(&mut self.text.author),
            "composer-text" => Some(&mut self.text.composer),
            "key-text" => Some(&mut self.text.key),
            "time-text" => Some(&mut self.text.time),
            "meter-text" => Some(&mut self.text.meter),
            "note-text" => Some(&mut self.text.note),
            "octave-text" => Some(&mut self.text.octave),
            "lyrics-text" => Some(&mut self.text.lyrics),
            "divider-text" => Some(&mut self.text.divider),
            "key-label" => Some(&mut self.label.key),
            "time-label" => Some(&mut self.label.time),
            "tempo-label" => Some(&mut self.label.tempo),
            "jump-label" => Some(&mut self.label.jump),
            "mark-label" => Some(&mut self.label.mark),
            "dynamic-label" => Some(&mut self.label.dynamic),
            "volta-text" => Some(&mut self.text.volta),
            "page-number-text" => Some(&mut self.text.page_number),
            _ => None,
        }
    }
}

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

    pub page_margin: Edges,
    pub col_padding: f32,
    pub min_col_width: f32,
    pub max_lyric_width: f32,
    pub voice_row_height: f32,
    pub lyric_row_height: f32,
    pub system_gap: f32,
    pub header_gap: f32,
    pub elements: ElementStyles,
}

impl Default for RenderOptions {
    fn default() -> Self {
        Self {
            page: PageSize::A4,
            solfa_font_size: 12.0,
            lyric_font_size: 10.0,
            octave_font_size: 7.0,
            title_font_size: 16.0,
            name_font_size: 13.0,
            meta_font_size: 13.0,
            marker_font_size: 11.0,
            page_margin: Edges::all(56.7),
            col_padding: 2.0,
            min_col_width: 12.0,
            max_lyric_width: 18.0,
            voice_row_height: 28.0,
            lyric_row_height: 22.0,
            system_gap: 18.0,
            header_gap: 20.0,
            elements: ElementStyles::default(),
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

            apply_edges(&mut self.page_margin, &page.margin);
        }

        for (name, style) in &directives.elements {
            if let Some(target) = self.elements.get_mut(name) {
                apply_element_style(target, &style.value);
            }
        }
    }
}

fn apply_element_style(target: &mut ElementStyle, source: &CoreElementStyle) {
    if let Some(family) = &source.font_family {
        target.font_family = Some(family.clone());
    }

    if let Some(size) = source.font_size {
        target.font_size = Some(size);
    }

    apply_edges(&mut target.margin, &source.margin);
    apply_edges(&mut target.padding, &source.padding);
}

fn apply_edges(target: &mut Edges, source: &EdgeValues) {
    if let Some(value) = source.top {
        target.top = value;
    }

    if let Some(value) = source.right {
        target.right = value;
    }

    if let Some(value) = source.bottom {
        target.bottom = value;
    }

    if let Some(value) = source.left {
        target.left = value;
    }
}

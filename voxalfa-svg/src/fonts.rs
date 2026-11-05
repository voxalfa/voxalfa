use std::{collections::BTreeMap, sync::Arc};

use skrifa::{
    FontRef, MetadataProvider,
    prelude::{LocationRef, Size},
};
use voxalfa_core::{
    ast::directives::{ElementStyle, STYLE_ELEMENTS},
    output::metrics::StringMetric,
};

use crate::{error::Result, options::RenderOptions};

pub const SOLFA_FONT: &[u8] = include_bytes!("../fonts/FiraSans-Solfa.ttf");
pub const TEXT_FONT: &[u8] = include_bytes!("../fonts/NotoSans-Regular.ttf");
pub const TEXT_BOLD_FONT: &[u8] = include_bytes!("../fonts/NotoSans-Bold.ttf");
pub const SERIF_ITALIC_FONT: &[u8] = include_bytes!("../fonts/NotoSerif-Italic.ttf");
pub const SERIF_BOLD_ITALIC_FONT: &[u8] = include_bytes!("../fonts/NotoSerif-BoldItalic.ttf");
pub const LYRIC_FONT: &[u8] = include_bytes!("../fonts/NotoSans-Lyrics.ttf");
pub const MUSIC_FONT: &[u8] = include_bytes!("../fonts/NotoSansSymbols-Bold.ttf");

pub const SOLFA_FAMILY: &str = "Fira Sans";
pub const MUSIC_FAMILY: &str = "Noto Sans Symbols";
pub const TEXT_FAMILY: &str = "Noto Sans";
pub const SERIF_FAMILY: &str = "Noto Serif";
pub const LYRIC_FAMILY: &str = "Noto Sans Condensed";

const SYMBOL_FONT_SCALE: f32 = 0.85;
const DYNAMIC_FONT_SCALE: f32 = 0.9;
const PAGE_NUMBER_FONT_SCALE: f32 = 0.9;

#[derive(Debug, Clone)]
pub struct FontFace {
    pub family: String,
    pub weight: u16,
    pub italic: bool,
    pub data: Arc<[u8]>,
}

impl FontFace {
    pub fn new(family: impl Into<String>, data: impl Into<Arc<[u8]>>) -> Self {
        Self {
            family: family.into(),
            weight: 400,
            italic: false,
            data: data.into(),
        }
    }

    pub fn bold(mut self) -> Self {
        self.weight = 700;
        self
    }

    pub fn italic(mut self) -> Self {
        self.italic = true;
        self
    }

    fn from_element(element: &str) -> Self {
        match element {
            "note-text" | "octave-text" | "divider-text" => {
                Self::new(SOLFA_FAMILY, SOLFA_FONT).bold()
            }
            "lyrics-text" => Self::new(LYRIC_FAMILY, LYRIC_FONT),
            "title-text" | "key-text" | "key-label" | "jump-label" => {
                Self::new(TEXT_FAMILY, TEXT_BOLD_FONT).bold()
            }
            "author-text" | "composer-text" | "meter-text" | "time-text" | "page-number-text"
            | "time-label" => Self::new(TEXT_FAMILY, TEXT_FONT),
            "mark-label" | "tempo-label" => Self::new(SERIF_FAMILY, SERIF_ITALIC_FONT).italic(),
            "dynamic-label" => Self::new(SERIF_FAMILY, SERIF_BOLD_ITALIC_FONT)
                .bold()
                .italic(),
            _ => Self::new(TEXT_FAMILY, TEXT_FONT),
        }
    }
}

#[derive(Debug, Clone)]
pub struct FontFaces {
    pub map: BTreeMap<String, FontFace>,
}

impl Default for FontFaces {
    fn default() -> Self {
        Self {
            map: STYLE_ELEMENTS
                .iter()
                .map(|&elem| (elem.to_string(), FontFace::from_element(elem)))
                .collect(),
        }
    }
}

pub struct FontMetrics {
    family: String,
    data: Arc<[u8]>,
    size: f32,
    ascent: f32,
    descent: f32,
}

impl FontMetrics {
    fn new(face: &FontFace, size: f32) -> Result<Self> {
        let font = FontRef::new(&face.data)?;
        let metrics = font.metrics(Size::new(size), LocationRef::default());

        Ok(Self {
            family: face.family.clone(),
            data: face.data.clone(),
            size,
            ascent: metrics.ascent,
            descent: metrics.descent.abs(),
        })
    }

    pub fn family(&self) -> &str {
        &self.family
    }

    pub fn size(&self) -> f32 {
        self.size
    }

    pub fn ascent(&self) -> f32 {
        self.ascent
    }

    pub fn descent(&self) -> f32 {
        self.descent
    }

    pub fn line_height(&self) -> f32 {
        self.ascent + self.descent
    }

    pub fn width(&self, text: &str) -> f32 {
        let Ok(font) = FontRef::new(&self.data) else {
            return 0.0;
        };

        let glyph_metrics = font.glyph_metrics(Size::new(self.size), LocationRef::default());
        let charmap = font.charmap();

        text.chars()
            .filter_map(|ch| charmap.map(ch))
            .filter_map(|id| glyph_metrics.advance_width(id))
            .sum()
    }
}

impl StringMetric for &FontMetrics {
    type Output = f32;

    fn measure_string(&self, text: &str) -> Self::Output {
        self.width(text)
    }
}

pub struct Fonts {
    pub text: TextFonts,
    pub label: LabelFonts,
    pub touch: FontMetrics,
    pub faces: Vec<FontFace>,
}

pub struct TextFonts {
    pub note: FontMetrics,
    pub octave: FontMetrics,
    pub lyric: FontMetrics,
    pub title: FontMetrics,
    pub author: FontMetrics,
    pub composer: FontMetrics,
    pub header_key: FontMetrics,
    pub time: FontMetrics,
    pub meter: FontMetrics,
    pub volta: FontMetrics,
    pub page_number: FontMetrics,
    pub divider: FontMetrics,
}

pub struct LabelFonts {
    pub key: FontMetrics,
    pub jump: FontMetrics,
    pub mark: FontMetrics,
    pub dynamic: FontMetrics,
    pub tempo: FontMetrics,
    pub time_signature: FontMetrics,
}

impl Fonts {
    pub fn with_faces(options: &RenderOptions, faces: &FontFaces) -> Result<Self> {
        todo!()
    }
}

impl LabelFonts {
    pub fn with_faces(options: &RenderOptions, faces: &FontFaces) -> Result<Self> {
        let styles = &options.elements.label;
        let default_size = options.marker_font_size;

        Ok(Self {
            key: make_metrics("key-label", default_size, faces, &styles.key)?,
            jump: make_metrics("jump-label", default_size, faces, &styles.key)?,
            mark: make_metrics("mark-label", default_size, faces, &styles.key)?,
            dynamic: make_metrics("dynamic-label", default_size, faces, &styles.key)?,
            tempo: make_metrics("tempo-label", default_size, faces, &styles.key)?,
            time_signature: make_metrics("time-label", default_size, faces, &styles.key)?,
        })
    }
}

fn make_metrics(
    key: &str,
    default_size: f32,
    faces: &FontFaces,
    style: &ElementStyle,
) -> Result<FontMetrics> {
    let face = &faces.map[key];
    let size = style.size(default_size);

    FontMetrics::new(&face, size)
}

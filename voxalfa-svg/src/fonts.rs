use std::sync::Arc;

use skrifa::{
    FontRef, MetadataProvider,
    prelude::{LocationRef, Size},
};
use voxalfa_core::output::metrics::StringMetric;

use crate::{
    error::Result,
    options::{ElementStyle, RenderOptions},
};

pub const SOLFA_FONT: &[u8] = include_bytes!("../fonts/FiraSans-Solfa.ttf");
pub const TEXT_FONT: &[u8] = include_bytes!("../fonts/NotoSans-Regular.ttf");
pub const TEXT_BOLD_FONT: &[u8] = include_bytes!("../fonts/NotoSans-Bold.ttf");
pub const SERIF_FONT: &[u8] = include_bytes!("../fonts/NotoSerif-Regular.ttf");
pub const SERIF_ITALIC_FONT: &[u8] = include_bytes!("../fonts/NotoSerif-Italic.ttf");
pub const SERIF_BOLD_ITALIC_FONT: &[u8] = include_bytes!("../fonts/NotoSerif-BoldItalic.ttf");
pub const LYRIC_FONT: &[u8] = include_bytes!("../fonts/NotoSans-Lyrics.ttf");

pub const SOLFA_FAMILY: &str = "voxalfa-solfa";
pub const TEXT_FAMILY: &str = "voxalfa-text";
pub const SERIF_FAMILY: &str = "voxalfa-serif";
pub const LYRIC_FAMILY: &str = "voxalfa-lyrics";

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
}

#[derive(Debug, Clone)]
pub struct FontFaces {
    pub solfa: FontFace,
    pub text: FontFace,
    pub text_bold: FontFace,
    pub serif: FontFace,
    pub serif_italic: FontFace,
    pub serif_bold_italic: FontFace,
    pub lyric: FontFace,
}

impl Default for FontFaces {
    fn default() -> Self {
        Self {
            solfa: FontFace::new(SOLFA_FAMILY, SOLFA_FONT),
            text: FontFace::new(TEXT_FAMILY, TEXT_FONT),
            text_bold: FontFace::new(TEXT_FAMILY, TEXT_BOLD_FONT).bold(),
            serif: FontFace::new(SERIF_FAMILY, SERIF_FONT),
            serif_italic: FontFace::new(SERIF_FAMILY, SERIF_ITALIC_FONT).italic(),
            serif_bold_italic: FontFace::new(SERIF_FAMILY, SERIF_BOLD_ITALIC_FONT)
                .bold()
                .italic(),
            lyric: FontFace::new(LYRIC_FAMILY, LYRIC_FONT),
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
        let mut builder = FontBuilder::default();
        let text = TextFonts::new(options, faces, &mut builder)?;
        let label = LabelFonts::new(options, faces, &mut builder)?;
        let touch = builder.fixed(&faces.serif, options.marker_font_size)?;

        Ok(Self {
            text,
            label,
            touch,
            faces: builder.faces,
        })
    }
}

impl TextFonts {
    fn new(opts: &RenderOptions, faces: &FontFaces, builder: &mut FontBuilder) -> Result<Self> {
        let styles = &opts.elements.text;
        let meta = opts.meta_font_size;

        Ok(Self {
            note: builder.metric(&faces.solfa, opts.solfa_font_size, &styles.note)?,
            octave: builder.metric(&faces.solfa, opts.octave_font_size, &styles.octave)?,
            lyric: builder.metric(&faces.lyric, opts.lyric_font_size, &styles.lyrics)?,
            title: builder.metric(&faces.text_bold, opts.title_font_size, &styles.title)?,
            author: builder.metric(&faces.text, opts.name_font_size, &styles.author)?,
            composer: builder.metric(&faces.text, opts.name_font_size, &styles.composer)?,
            header_key: builder.metric(&faces.text_bold, opts.marker_font_size, &styles.key)?,
            time: builder.metric(&faces.text, meta, &styles.time)?,
            meter: builder.metric(&faces.text, meta, &styles.meter)?,
            volta: builder.metric(&faces.text_bold, opts.marker_font_size, &styles.volta)?,
            page_number: builder.metric(
                &faces.text,
                meta * PAGE_NUMBER_FONT_SCALE,
                &styles.page_number,
            )?,
            divider: builder.metric(
                &faces.solfa,
                opts.solfa_font_size * SYMBOL_FONT_SCALE,
                &styles.divider,
            )?,
        })
    }
}

impl LabelFonts {
    fn new(options: &RenderOptions, faces: &FontFaces, builder: &mut FontBuilder) -> Result<Self> {
        let styles = &options.elements.label;
        let marker = options.marker_font_size;

        Ok(Self {
            key: builder.metric(&faces.text_bold, marker, &styles.key)?,
            jump: builder.metric(&faces.text_bold, marker, &styles.jump)?,
            mark: builder.metric(&faces.serif_italic, marker, &styles.mark)?,
            dynamic: builder.metric(
                &faces.serif_bold_italic,
                marker * DYNAMIC_FONT_SCALE,
                &styles.dynamic,
            )?,
            tempo: builder.metric(&faces.serif_italic, marker, &styles.tempo)?,
            time_signature: builder.metric(&faces.text, marker, &styles.time)?,
        })
    }
}

#[derive(Default)]
struct FontBuilder {
    faces: Vec<FontFace>,
}

impl FontBuilder {
    fn metric(
        &mut self,
        default: &FontFace,
        default_size: f32,
        style: &ElementStyle,
    ) -> Result<FontMetrics> {
        let face = match &style.font_family {
            Some(family) => FontFace {
                family: family.clone(),
                ..default.clone()
            },
            None => default.clone(),
        };

        let metrics = FontMetrics::new(&face, style.size(default_size))?;
        self.register(&face);

        Ok(metrics)
    }

    fn fixed(&mut self, face: &FontFace, size: f32) -> Result<FontMetrics> {
        let metrics = FontMetrics::new(face, size)?;
        self.register(face);

        Ok(metrics)
    }

    fn register(&mut self, face: &FontFace) {
        let exists = self.faces.iter().any(|current| {
            current.family == face.family
                && current.weight == face.weight
                && current.italic == face.italic
        });

        if !exists {
            self.faces.push(face.clone());
        }
    }
}

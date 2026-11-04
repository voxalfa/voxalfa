use skrifa::{
    FontRef, MetadataProvider,
    prelude::{LocationRef, Size},
};
use voxalfa_core::output::metrics::StringMetric;

use crate::{error::Result, options::RenderOptions};

pub const SOLFA_FONT: &[u8] = include_bytes!("../fonts/FiraSans-Solfa.ttf");
pub const LYRIC_FONT: &[u8] = include_bytes!("../fonts/NotoSans-Lyrics.ttf");

pub const SOLFA_FAMILY: &str = "voxalfa-solfa";
pub const LYRIC_FAMILY: &str = "voxalfa-lyrics";

pub struct FontMetrics {
    data: &'static [u8],
    size: f32,
    ascent: f32,
    descent: f32,
}

impl FontMetrics {
    pub fn new(data: &'static [u8], size: f32) -> Result<Self> {
        let font = FontRef::new(data)?;
        let metrics = font.metrics(Size::new(size), LocationRef::default());

        Ok(Self {
            data,
            size,
            ascent: metrics.ascent,
            descent: metrics.descent.abs(),
        })
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
        let Ok(font) = FontRef::new(self.data) else {
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
    pub solfa: FontMetrics,
    pub lyric: FontMetrics,
    pub octave: FontMetrics,
    pub title: FontMetrics,
    pub name: FontMetrics,
    pub meta: FontMetrics,
}

impl Fonts {
    pub fn new(options: &RenderOptions) -> Result<Self> {
        Ok(Self {
            solfa: FontMetrics::new(SOLFA_FONT, options.solfa_font_size)?,
            lyric: FontMetrics::new(LYRIC_FONT, options.lyric_font_size)?,
            octave: FontMetrics::new(SOLFA_FONT, options.octave_font_size)?,
            title: FontMetrics::new(LYRIC_FONT, options.title_font_size)?,
            name: FontMetrics::new(LYRIC_FONT, options.name_font_size)?,
            meta: FontMetrics::new(LYRIC_FONT, options.meta_font_size)?,
        })
    }
}

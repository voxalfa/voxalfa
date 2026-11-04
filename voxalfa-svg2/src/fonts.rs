use skrifa::{
    FontRef, MetadataProvider,
    prelude::{LocationRef, Size},
};

use crate::{error::Result, options::RenderOptions};

pub const SOLFA_FONT: &[u8] = include_bytes!("../fonts/FiraSans-Solfa.ttf");
pub const LYRIC_FONT: &[u8] = include_bytes!("../fonts/NotoSans-Lyrics.ttf");

pub const SOLFA_FAMILY: &str = "Voxalfa-Solfa";
pub const LYRIC_FAMILY: &str = "Voxalfa-Lyrics";

/// A single loaded font at a fixed size.
///
/// `skrifa::FontRef` borrows its input, so keeping the bytes in the struct and
/// creating a `FontRef` on demand avoids self-referential lifetime tricks.
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

    /// Advance-width measurement.
    ///
    /// The current implementation deliberately uses simple advance sums: the
    /// embedded fonts are monospaced for solfa and the lyric font does not need
    /// kerning for the supported scripts.
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

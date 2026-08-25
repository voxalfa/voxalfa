use skrifa::{
    FontRef, MetadataProvider,
    prelude::{LocationRef, Size},
};
use voxalfa_core::output::metrics::StringMetric;

use crate::error::Result;

pub const SOLFA_FONT: &[u8] = include_bytes!("../fonts/FiraSans-Solfa.ttf");
pub const LYRIC_FONT: &[u8] = include_bytes!("../fonts/NotoSans-Lyrics.ttf");

pub const SOLFA_FONT_SIZE: f32 = 14.0;
pub const LYRIC_FONT_SIZE: f32 = 14.0;
pub const OCTAVE_FONT_SIZE: f32 = 10.0;

pub struct FontInterface<'a> {
    pub solfa: FontMeasurer<'a>,
    pub lyric: FontMeasurer<'a>,
}

impl FontInterface<'_> {
    pub fn new() -> Result<Self> {
        Ok(Self {
            solfa: FontMeasurer::new(SOLFA_FONT, SOLFA_FONT_SIZE)?,
            lyric: FontMeasurer::new(LYRIC_FONT, LYRIC_FONT_SIZE)?,
        })
    }
}

impl StringMetric for &FontInterface<'_> {
    type Output = f32;

    fn measure_string(&self, text: &str) -> Self::Output {
        self.lyric.get_width(text)
    }
}

pub struct FontMeasurer<'a> {
    pub size: Size,
    pub font: FontRef<'a>,
    pub ascent: f32,
    pub descent: f32,
}

impl<'a> FontMeasurer<'a> {
    pub fn get_width(&self, text: &str) -> f32 {
        let glyph_metrics = self.font.glyph_metrics(self.size, LocationRef::default());
        let charmap = self.font.charmap();

        text.chars()
            .filter_map(|ch| charmap.map(ch))
            .filter_map(|id| glyph_metrics.advance_width(id))
            .sum()
    }

    fn new(font_bytes: &'a [u8], font_size: f32) -> Result<Self> {
        let font = FontRef::new(font_bytes)?;
        let size = Size::new(font_size);
        let metrics = font.metrics(size, LocationRef::default());
        let ascent = metrics.ascent;
        let descent = metrics.descent.abs();

        Ok(Self {
            font,
            size,
            ascent,
            descent,
        })
    }
}

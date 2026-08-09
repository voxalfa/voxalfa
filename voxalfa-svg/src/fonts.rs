use skrifa::{
    FontRef, MetadataProvider,
    prelude::{LocationRef, Size},
};

use crate::error::Result;

pub const SOLFA_FONT: &[u8] = include_bytes!("../fonts/FiraSans-Solfa.ttf");
pub const LYRIC_FONT: &[u8] = include_bytes!("../fonts/NotoSans-Lyrics.ttf");

pub const SOLFA_FONT_SIZE: f32 = 16.0;
pub const LYRIC_FONT_SIZE: f32 = 14.0;

pub struct FontInterface<'a> {
    solfa: FontMeasurer<'a>,
    lyric: FontMeasurer<'a>,
}

impl FontInterface<'_> {
    pub fn new() -> Result<Self> {
        Ok(Self {
            solfa: FontMeasurer::new(SOLFA_FONT, SOLFA_FONT_SIZE)?,
            lyric: FontMeasurer::new(LYRIC_FONT, LYRIC_FONT_SIZE)?,
        })
    }

    pub fn measure_solfa(&self, text: &str) -> f32 {
        self.solfa.get_width(text)
    }

    pub fn measure_lyric(&self, text: &str) -> f32 {
        self.lyric.get_width(text)
    }
}

struct FontMeasurer<'a> {
    font: FontRef<'a>,
    size: Size,
}

impl<'a> FontMeasurer<'a> {
    fn get_width(&self, text: &str) -> f32 {
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

        Ok(Self { font, size })
    }
}

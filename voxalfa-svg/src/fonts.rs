use skrifa::{
    FontRef, MetadataProvider,
    prelude::{LocationRef, Size},
};
use voxalfa_core::output::metrics::StringMetric;

use crate::{error::Result, layout::DYNAMIC_FONT_SCALE, options::RenderOptions};

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
    pub key: FontMetrics,
    pub meta: FontMetrics,
    pub marker: FontMetrics,
    pub marker_sans: FontMetrics,
    pub marker_bold: FontMetrics,
    pub mark: FontMetrics,
    pub dynamic: FontMetrics,
}

impl Fonts {
    pub fn new(options: &RenderOptions) -> Result<Self> {
        Ok(Self {
            solfa: FontMetrics::new(SOLFA_FONT, options.solfa_font_size)?,
            lyric: FontMetrics::new(LYRIC_FONT, options.lyric_font_size)?,
            octave: FontMetrics::new(SOLFA_FONT, options.octave_font_size)?,
            title: FontMetrics::new(TEXT_BOLD_FONT, options.title_font_size)?,
            name: FontMetrics::new(TEXT_FONT, options.name_font_size)?,
            key: FontMetrics::new(TEXT_BOLD_FONT, options.marker_font_size)?,
            meta: FontMetrics::new(TEXT_FONT, options.meta_font_size)?,
            marker: FontMetrics::new(SERIF_FONT, options.marker_font_size)?,
            marker_sans: FontMetrics::new(TEXT_FONT, options.marker_font_size)?,
            marker_bold: FontMetrics::new(TEXT_BOLD_FONT, options.marker_font_size)?,
            mark: FontMetrics::new(SERIF_ITALIC_FONT, options.marker_font_size)?,
            dynamic: FontMetrics::new(
                SERIF_BOLD_ITALIC_FONT,
                options.marker_font_size * DYNAMIC_FONT_SCALE,
            )?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subsets_cover_header_and_marker_text() {
        let meta = FontMetrics::new(TEXT_FONT, 13.0).unwrap();
        let bold = FontMetrics::new(TEXT_BOLD_FONT, 13.0).unwrap();
        let marker = FontMetrics::new(SERIF_FONT, 13.0).unwrap();
        let mark = FontMetrics::new(SERIF_ITALIC_FONT, 13.0).unwrap();
        let italic = FontMetrics::new(SERIF_BOLD_ITALIC_FONT, 13.0).unwrap();

        for text in ["Dô dia", "Do est", "D.S. al Coda", "To Coda", "’"] {
            assert!(meta.width(text) > 0.0, "missing text glyphs: {text}");
        }

        assert!(bold.width("B♭") > 0.0);
        assert!(bold.width("D.S. al Coda") > 0.0);
        assert!(marker.width("D.S. al Coda") > 0.0);
        assert!(mark.width("Fine") > 0.0);
        assert!(italic.width("mf") > 0.0);
    }
}

/// Physical size of the output page, in SVG user units (1 unit = 1 px at 96 DPI).
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

/// Rendering configuration.
///
/// Everything that affects the visual output is kept here so a caller can
/// produce a completely different theme without touching the renderer.
#[derive(Debug, Clone)]
pub struct RenderOptions {
    pub page: PageSize,
    /// Page margin on every side.
    pub margin: f32,

    pub solfa_font_size: f32,
    pub lyric_font_size: f32,
    pub octave_font_size: f32,
    pub title_font_size: f32,
    pub name_font_size: f32,
    pub meta_font_size: f32,

    /// Extra horizontal padding added to the widest lyric/note column.
    pub col_padding: f32,
    /// Lower bound for a single note column.
    pub min_col_width: f32,
    /// Upper bound for the lyric part of a column. A long syllable is allowed
    /// to overflow its grid cell so that a single word does not widen every
    /// pulse in the score.
    pub max_lyric_width: f32,

    /// Height of one voice line.
    pub voice_row_height: f32,
    /// Height of one lyric line.
    pub lyric_row_height: f32,
    /// Vertical gap between two systems.
    pub system_gap: f32,
    /// Vertical gap between the header and the first system.
    pub header_gap: f32,
}

impl Default for RenderOptions {
    fn default() -> Self {
        Self {
            page: PageSize::A4,
            margin: 56.7,
            solfa_font_size: 12.0,
            lyric_font_size: 10.0,
            octave_font_size: 7.0,
            title_font_size: 26.0,
            name_font_size: 14.0,
            meta_font_size: 13.0,
            col_padding: 2.0,
            min_col_width: 12.0,
            max_lyric_width: 18.0,
            voice_row_height: 28.0, //
            lyric_row_height: 22.0, // FIXME: should be relative to font size
            system_gap: 18.0,
            header_gap: 20.0,
        }
    }
}

impl RenderOptions {
    pub fn printable_width(&self) -> f32 {
        (self.page.width - self.margin * 2.0).max(0.0)
    }

    pub fn printable_height(&self) -> f32 {
        (self.page.height - self.margin * 2.0).max(0.0)
    }
}

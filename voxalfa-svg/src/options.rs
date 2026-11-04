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

    pub min_margin: f32,
    pub col_padding: f32,
    pub min_col_width: f32,
    pub max_lyric_width: f32,
    pub voice_row_height: f32,
    pub lyric_row_height: f32,
    pub system_gap: f32,
    pub header_gap: f32,
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
            min_margin: 56.7,
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
        (self.page.width - self.min_margin * 2.0).max(0.0)
    }

    pub fn printable_height(&self) -> f32 {
        (self.page.height - self.min_margin * 2.0).max(0.0)
    }
}

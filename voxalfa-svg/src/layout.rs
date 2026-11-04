pub const TITLE_LINE_HEIGHT_FACTOR: f32 = 1.08;
pub const HEADER_GROUP_GAP: f32 = 6.0;
pub const HEADER_BOTTOM_PADDING: f32 = 8.0;
pub const HEADER_TITLE_BOTTOM_GAP: f32 = 12.0;
pub const HEADER_KEY_RIGHT_MARGIN: f32 = 28.0;
pub const HEADER_RIGHT_INSET: f32 = 12.0;
pub const MARKER_BOTTOM_GAP: f32 = 4.0;
pub const MARKER_HORIZONTAL_GAP: f32 = 6.0;

pub const VOLTA_TOP_GAP: f32 = 6.0;
pub const VOLTA_TEXT_GAP: f32 = 2.0;
pub const VOLTA_BOTTOM_GAP: f32 = 4.0;
pub const VOLTA_END_TICK: f32 = 5.0;
pub const VOLTA_TEXT_INSET: f32 = 2.0;

pub const SYMBOL_FONT_SCALE: f32 = 0.85;
pub const TOUCH_SYMBOL_SCALE: f32 = 0.75;
pub const MARCATO_WIDTH_SCALE: f32 = 0.85;
pub const ACCENT_STROKE_WIDTH: f32 = 1.0;

pub const DYNAMIC_SPAN_INSET: f32 = 10.0;
pub const DYNAMIC_FONT_SCALE: f32 = 0.9;
pub const DYNAMIC_HAIRPIN_HEIGHT: f32 = 5.0;
pub const DYNAMIC_DASH_GAP: f32 = 3.0;
pub const DYNAMIC_SPAN_BOTTOM_GAP: f32 = 2.0;

pub const BARLINE_STROKE_WIDTH: f32 = 1.2;
pub const BARLINE_BLOCK_INSET: f32 = 6.0;
pub const DOUBLE_BARLINE_GAP: f32 = 3.0;

pub const OCTAVE_GAP: f32 = 2.0;
pub const TOP_OCTAVE_OFFSET_FACTOR: f32 = 0.45;
pub const UNDERLINE_STROKE_WIDTH: f32 = 1.0;
pub const BOTTOM_OCTAVE_UNDERLINE_GAP: f32 = 1.0;
pub const SOLFA_UNDERLINE_OFFSET: f32 = 2.5;
pub const LYRIC_UNDERLINE_OFFSET: f32 = 1.0;
pub const MIN_UNDERLINE_WIDTH: f32 = 1.0;
pub const PROLONGED_LEADING_MIN: f32 = 2.5;
pub const NOTE_BARLINE_CLEARANCE: f32 = 3.0;
pub const GRID_RIGHT_PADDING: f32 = 2.0;

pub const PAGE_NUMBER_FONT_SCALE: f32 = 0.9;

pub(crate) struct RowSpec {
    pub height: f32,
    pub gap_before: f32,
}

pub(crate) struct ColumnLayout {
    pub ys: Vec<f32>,
    pub height: f32,
}

pub(crate) fn layout_column(rows: &[RowSpec]) -> ColumnLayout {
    let mut ys = Vec::with_capacity(rows.len());
    let mut y = 0.0;

    for row in rows {
        y += row.gap_before;
        ys.push(y);
        y += row.height;
    }

    ColumnLayout { ys, height: y }
}

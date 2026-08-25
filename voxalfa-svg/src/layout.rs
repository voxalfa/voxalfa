pub const A4_WIDTH_PX: f32 = 794.0; // 210mm at 96 DPI
pub const A4_HEIGHT_PX: f32 = 1123.0; // 297mm at 96 DPI
pub const A4_PADDING: f32 = 56.7; // 15mm padding

pub const PRINTABLE_WIDTH: f32 = A4_WIDTH_PX - (A4_PADDING * 2.0); // 680.6px

pub const VOICE_LINE_HEIGHT: f32 = 24.0; // Vertical distance between S, A, T, B
pub const LYRIC_LINE_HEIGHT: f32 = 20.0; // Vertical distance per lyric verse

pub const LINE_GAP: f32 = 10.0;
pub const SYSTEM_GAP: f32 = 25.0;

pub const UNDERLINE_Y_OFFSET: f32 = 3.0;

// FIXME: should be dynamic
pub const OCTAVE_TOP_OFFSET: f32 = 10.0;
pub const OCTAVE_BOTTOM_OFFSET: f32 = 2.0;

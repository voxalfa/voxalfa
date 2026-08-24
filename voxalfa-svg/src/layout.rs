pub const A4_WIDTH_PX: f32 = 794.0; // 210mm at 96 DPI
pub const A4_HEIGHT_PX: f32 = 1123.0; // 297mm at 96 DPI
pub const MARGIN_X: f32 = 56.7; // 15mm left/right margin
pub const MARGIN_Y: f32 = 56.7; // 15mm top/bottom margin

pub const PRINTABLE_WIDTH: f32 = A4_WIDTH_PX - (MARGIN_X * 2.0); // 680.6px

pub const VOICE_LINE_HEIGHT: f32 = 24.0; // Vertical distance between S, A, T, B
pub const LYRIC_LINE_HEIGHT: f32 = 20.0; // Vertical distance per lyric verse
pub const SYSTEM_GAP: f32 = 40.0; // Vertical distance between system blocks

use async_lsp::lsp_types::{self, Position as LspPosition};
use voxalfa_validator::ts_utils::range::{Position, Range};

pub fn convert_range(range: &Range) -> lsp_types::Range {
    let start_point = range.start_point;
    let end_point = range.end_point;
    let start_pos = LspPosition::new(start_point.row as u32, start_point.column as u32);
    let end_pos = LspPosition::new(end_point.row as u32, end_point.column as u32);

    lsp_types::Range::new(start_pos, end_pos)
}

pub fn convert_position(position: LspPosition) -> Position {
    Position {
        line: position.line as usize,
        column: position.character as usize,
    }
}

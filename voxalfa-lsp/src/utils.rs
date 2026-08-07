use async_lsp::lsp_types;
use voxalfa_validator::ts_utils::range::{Position, Range};

pub fn ts_range_to_lsp(range: &Range) -> lsp_types::Range {
    let start_point = range.start_point;
    let end_point = range.end_point;
    let start_pos = lsp_types::Position::new(start_point.row as u32, start_point.column as u32);
    let end_pos = lsp_types::Position::new(end_point.row as u32, end_point.column as u32);

    lsp_types::Range::new(start_pos, end_pos)
}

pub fn lsp_pos_to_ts(position: lsp_types::Position) -> Position {
    Position {
        row: position.line as usize,
        column: position.character as usize,
    }
}

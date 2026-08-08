use async_lsp::lsp_types;
use ropey::Rope;
use voxalfa_validator::ts_utils::range::{Position, Range};

pub fn ts_pos_to_lsp(rope: &Rope, pos: Position) -> lsp_types::Position {
    let line = rope.line(pos.row);
    let byte_col = pos.column.min(line.len_bytes());
    let char_idx = line.byte_to_char(byte_col);
    let utf16_col = line.char_to_utf16_cu(char_idx);

    lsp_types::Position::new(pos.row as u32, utf16_col as u32)
}

pub fn ts_range_to_lsp(rope: &Rope, range: &Range) -> lsp_types::Range {
    let start_pos = ts_pos_to_lsp(rope, range.start_point);
    let end_pos = ts_pos_to_lsp(rope, range.end_point);

    lsp_types::Range::new(start_pos, end_pos)
}

pub fn lsp_pos_to_ts(rope: &Rope, position: lsp_types::Position) -> Position {
    let line = rope.line(position.line as usize);
    let utf16_col = (position.character as usize).min(line.len_utf16_cu());
    let char_idx = line.utf16_cu_to_char(utf16_col);
    let byte_col = line.char_to_byte(char_idx);

    Position {
        row: position.line as usize,
        column: byte_col,
    }
}

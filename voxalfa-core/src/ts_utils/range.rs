pub use tree_sitter::{Point as Position, Range};

pub trait RangeUtil {
    fn start(&self) -> Range;
    fn end(&self) -> Range;
    fn merge(&self, other: Self) -> Range;
    fn line(&self) -> usize;
    fn contains(&self, pos: &Position) -> bool;
    fn overlaps(&self, pos: &Position) -> bool;
}

impl RangeUtil for Range {
    fn merge(&self, other: Self) -> Range {
        Range {
            start_byte: self.start_byte,
            end_byte: other.end_byte,
            start_point: self.start_point,
            end_point: other.end_point,
        }
    }

    fn start(&self) -> Range {
        Range {
            start_byte: self.start_byte,
            end_byte: self.start_byte,
            start_point: self.start_point,
            end_point: self.start_point,
        }
    }

    fn end(&self) -> Range {
        Range {
            start_byte: self.end_byte,
            end_byte: self.end_byte,
            start_point: self.end_point,
            end_point: self.end_point,
        }
    }

    fn line(&self) -> usize {
        self.start_point.row
    }

    fn contains(&self, pos: &Position) -> bool {
        let start = self.start_point;
        let end = self.end_point;
        *pos >= start && *pos <= end
    }

    fn overlaps(&self, pos: &Position) -> bool {
        let start = self.start_point;
        let end = self.end_point;
        *pos >= start && *pos < end
    }
}

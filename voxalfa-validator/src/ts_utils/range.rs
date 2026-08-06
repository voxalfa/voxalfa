pub use tree_sitter::{Point, Range};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Position {
    pub line: usize,
    pub column: usize,
}

impl Position {
    pub fn new(line: usize, column: usize) -> Self {
        Position { line, column }
    }
}

impl From<Point> for Position {
    fn from(point: Point) -> Self {
        Self {
            line: point.row,
            column: point.column,
        }
    }
}

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
        let start = Position::from(self.start_point);
        let end = Position::from(self.end_point);
        *pos >= start && *pos <= end
    }

    fn overlaps(&self, pos: &Position) -> bool {
        let start = Position::from(self.start_point);
        let end = Position::from(self.end_point);
        *pos >= start && *pos < end
    }
}

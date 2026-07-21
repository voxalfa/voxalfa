pub type Range = tree_sitter::Range;

pub trait RangeUtil {
    fn start(&self) -> Range;
    fn end(&self) -> Range;
    fn merge(&self, other: Self) -> Range;
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
}

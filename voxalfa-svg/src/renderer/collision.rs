#[derive(Debug, Clone, Copy)]
pub struct Rect {
    pub left: f32,
    pub right: f32,
    pub top: f32,
    pub bottom: f32,
}

impl Rect {
    pub fn intersects(&self, other: &Self) -> bool {
        self.horizontal_intersects(other) && self.vertical_intersects(other)
    }

    pub fn horizontal_intersects(&self, other: &Self) -> bool {
        self.left < other.right && self.right > other.left
    }

    pub fn vertical_intersects(&self, other: &Self) -> bool {
        self.top < other.bottom && self.bottom > other.top
    }

    pub fn shift_above(&mut self, obstacle: &Self, gap: f32) -> f32 {
        let shift = self.bottom - obstacle.top + gap;
        self.top -= shift;
        self.bottom -= shift;
        shift
    }
}

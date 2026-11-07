pub trait SpanWidth: Copy {
    fn get_average_width(width: Self, ratio: f32) -> Self;
}

impl SpanWidth for f32 {
    fn get_average_width(width: Self, ratio: f32) -> Self {
        width / ratio
    }
}

impl SpanWidth for usize {
    fn get_average_width(width: Self, ratio: f32) -> Self {
        (width as f32 / ratio).ceil() as usize
    }
}

impl SpanWidth for () {
    fn get_average_width(_width: Self, _ratio: f32) -> Self {}
}

pub trait StringMetric {
    type Output: Default + PartialOrd + Copy + SpanWidth + std::fmt::Debug;

    fn measure_string(&self, s: &str) -> Self::Output;
}

pub struct DummyMeasurer {}

impl StringMetric for DummyMeasurer {
    type Output = ();

    fn measure_string(&self, _s: &str) -> Self::Output {}
}

pub trait SpanWidth: Copy {
    fn per_column(width: Self, span: usize) -> Self;
}

impl SpanWidth for f32 {
    fn per_column(width: Self, span: usize) -> Self {
        width / span as f32
    }
}

impl SpanWidth for usize {
    fn per_column(width: Self, span: usize) -> Self {
        width.div_ceil(span)
    }
}

impl SpanWidth for () {
    fn per_column(_width: Self, _span: usize) -> Self {}
}

pub trait StringMetric {
    type Output: Default + PartialOrd + Copy + SpanWidth;

    fn measure_string(&self, s: &str) -> Self::Output;
}

pub struct DummyMeasurer {}

impl StringMetric for DummyMeasurer {
    type Output = ();

    fn measure_string(&self, _s: &str) -> Self::Output {}
}

pub trait SpanWidth: Copy {
    fn per_column(width: Self, units: f32) -> Self;
}

impl SpanWidth for f32 {
    fn per_column(width: Self, units: f32) -> Self {
        width / units.max(1.0)
    }
}

impl SpanWidth for usize {
    fn per_column(width: Self, units: f32) -> Self {
        (width as f32 / units.max(1.0)).ceil() as usize
    }
}

impl SpanWidth for () {
    fn per_column(_width: Self, _units: f32) -> Self {}
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

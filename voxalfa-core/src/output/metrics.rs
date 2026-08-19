pub trait StringMetric {
    type Output: Default + Ord + Copy;

    fn measure_string(&self, s: &str) -> Self::Output;
}

pub struct DummyMeasurer {}

impl StringMetric for DummyMeasurer {
    type Output = ();

    fn measure_string(&self, _s: &str) -> Self::Output {}
}

use voxalfa_core::output::metrics::StringMetric;

pub struct CharMeasurer {}

impl StringMetric for CharMeasurer {
    type Output = usize;

    fn measure_string(&self, s: &str) -> Self::Output {
        s.chars().count()
    }
}

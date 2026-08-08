use crate::data_types::ProgressiveTempo;

#[derive(Debug, Default)]
pub struct TempoState {
    pub update: bool,
    pub transition: Option<TempoTransition>,
}

#[derive(Debug)]
pub struct TempoTransition {
    pub kind: ProgressiveTempo,
    pub initial_value: u16,
}

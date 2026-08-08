use crate::data_types::Dynamic;

#[derive(Debug)]
pub struct DynamicState {
    pub current: Dynamic,
    pub transition: Option<DynamicTransition>,
    pub update: bool,
}

impl Default for DynamicState {
    fn default() -> Self {
        Self {
            current: Dynamic::MF,
            transition: None,
            update: false,
        }
    }
}

#[derive(Debug)]
pub struct DynamicTransition {
    pub level: Dynamic,
    pub kind: DynamicTransitionKind,
}

#[derive(Debug)]
pub enum DynamicTransitionKind {
    Cre,
    Dec,
}

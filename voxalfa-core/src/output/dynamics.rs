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

impl DynamicTransition {
    pub fn target(&self, current: Dynamic) -> Dynamic {
        if current != self.level {
            return current;
        }

        let target = match self.kind {
            DynamicTransitionKind::Cre => self.level.get_next(),
            DynamicTransitionKind::Dec => self.level.get_prev(),
        };

        target.unwrap_or(self.level)
    }
}

#[derive(Debug, Clone, Copy)]
pub enum DynamicTransitionKind {
    Cre,
    Dec,
}

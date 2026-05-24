use crate::ast::{dynamics::Dynamics, params::SectionParams, solfa::SolfaLine, symbols::ScopeId};

#[derive(Debug, Default)]
pub struct Body {
    pub sid: ScopeId,
    pub sections: Vec<Section>,
}

impl Body {
    pub fn new(sid: ScopeId) -> Self {
        Self {
            sid,
            ..Default::default()
        }
    }
}

#[derive(Debug, Default)]
pub struct Section {
    pub sid: ScopeId,
    pub params: SectionParams,
    pub dynamics: Dynamics,
    pub solfa: Vec<SolfaLine>,
    // pub lyrics: Vec<LyricLine>,
}

impl Section {
    pub fn new(sid: ScopeId) -> Self {
        Self {
            sid,
            ..Default::default()
        }
    }
}

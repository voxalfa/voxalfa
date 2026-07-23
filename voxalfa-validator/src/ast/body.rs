use crate::ast::{
    dynamics::Dynamics, lyrics::LyricLine, params::CompositionParams, solfa::SolfaLine,
    symbols::ScopeId,
};

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
    pub items: Vec<SubSection>,
    pub params: CompositionParams,
    pub merge: bool,
}

impl Section {
    pub fn new(sid: ScopeId) -> Self {
        Self {
            sid,
            ..Default::default()
        }
    }
}

#[derive(Debug, Default)]
pub struct SubSection {
    pub id: usize,
    pub sid: ScopeId,
    pub dynamics: Dynamics,
    pub solfa: Vec<SolfaLine>,
    pub lyrics: Vec<LyricLine>,
}

impl SubSection {
    pub fn new(sid: ScopeId) -> Self {
        Self {
            sid,
            ..Default::default()
        }
    }
}

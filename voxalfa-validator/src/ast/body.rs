use crate::{
    ast::{
        dynamics::Dynamics,
        lyrics::LyricLine,
        params::CompositionParams,
        parser::Parser,
        solfa::SolfaLine,
        symbols::{Field, FieldAssign, ScopeId},
        types::Mark,
    },
    diagnostics::types::DiagnosticKind,
    ts_utils::types::AssignmentData,
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
    pub metadata: SectionMetadata,
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
pub struct SectionMetadata {
    pub name: Field<String>,
    pub ending: Field<usize>,
    pub head_mark: Field<Vec<Mark>>,
    pub tail_mark: Field<Vec<Mark>>,
}

impl FieldAssign for SectionMetadata {
    fn assign_field(&mut self, data: AssignmentData, context: &mut Parser) {
        match data.key_name.as_str() {
            "name" => context.assign_field(data, &mut self.name),
            "ending" => context.assign_field(data, &mut self.ending),
            "head-mark" => context.assign_field(data, &mut self.head_mark),
            "tail-mark" => context.assign_field(data, &mut self.tail_mark),
            _ => {
                context.reporter.error(
                    data.full_range,
                    DiagnosticKind::UnknownField(data.key_name.clone()),
                );
            }
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

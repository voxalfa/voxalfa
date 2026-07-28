use crate::{
    ast::{
        lyrics::LyricLine,
        params::{SectionParams, SubSectionParams},
        parser::Parser,
        solfa::SolfaLine,
        symbols::{Field, FieldAssign, ScopeId},
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
    pub params: SectionParams,
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
}

impl FieldAssign for SectionMetadata {
    fn assign_field(&mut self, data: AssignmentData, context: &mut Parser) {
        match data.key_name.as_str() {
            "name" => context.assign_field(data, &mut self.name),
            "ending" => context.assign_field(data, &mut self.ending),
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
    pub params: SubSectionParams,
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

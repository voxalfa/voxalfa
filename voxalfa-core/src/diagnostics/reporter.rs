use crate::{
    diagnostics::types::{Diagnostic, DiagnosticKind, DiagnosticLevel, ReportStage},
    ts_utils::range::Range,
};

#[derive(Debug)]
pub struct DiagnosticReporter {
    diagnostics: Vec<Diagnostic>,
    stage: ReportStage,
}

impl DiagnosticReporter {
    pub fn new(stage: ReportStage) -> Self {
        Self {
            stage,
            diagnostics: Vec::new(),
        }
    }
    pub fn error(&mut self, range: Range, kind: DiagnosticKind) {
        self.diagnostics.push(Diagnostic {
            level: DiagnosticLevel::Error,
            stage: self.stage,
            kind,
            range,
        })
    }

    pub fn warning(&mut self, range: Range, kind: DiagnosticKind) {
        self.diagnostics.push(Diagnostic {
            level: DiagnosticLevel::Warning,
            stage: self.stage,
            kind,
            range,
        });
    }

    pub fn merge(mut self, other: Self) -> Self {
        self.diagnostics.extend(other.diagnostics);
        self
    }

    pub fn into_diagnostics_vec(self) -> Vec<Diagnostic> {
        self.diagnostics
    }
}

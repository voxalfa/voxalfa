use crate::{
    diagnostic::{Diagnostic, DiagnosticKind, DiagnosticLevel},
    ts_utils::range::Range,
};

#[derive(Debug, Default)]
pub struct DiagnosticReporter {
    pub diagnostics: Vec<Diagnostic>,
}

impl DiagnosticReporter {
    pub fn error(&mut self, range: Range, kind: DiagnosticKind) {
        self.diagnostics.push(Diagnostic {
            level: DiagnosticLevel::Error,
            kind,
            range,
        })
    }

    pub fn warning(&mut self, range: Range, kind: DiagnosticKind) {
        self.diagnostics.push(Diagnostic {
            level: DiagnosticLevel::Warning,
            kind,
            range,
        });
    }
}

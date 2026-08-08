use async_lsp::lsp_types::{
    self, DiagnosticRelatedInformation, DiagnosticSeverity, Location, NumberOrString,
};
use voxalfa_core::diagnostics::types::{Diagnostic, DiagnosticLevel, DiagnosticRelatedInfo};

use crate::{SERVER_NAME, state::Document, utils::ts_range_to_lsp};

pub fn convert_diagnostic(doc: &Document, source: &Diagnostic) -> Vec<lsp_types::Diagnostic> {
    let mut result = Vec::new();

    let range = ts_range_to_lsp(&doc.rope, &source.range);
    let severity = convert_diagnostic_level(source.level);
    let raw_code = source.kind.get_code().to_string();
    let code = NumberOrString::String(raw_code).into();
    let extra_info = source.kind.get_extra_info();
    let related_information = get_related_info(doc, &extra_info);

    let main = lsp_types::Diagnostic {
        range,
        severity: Some(severity),
        code,
        code_description: None,
        source: SERVER_NAME.to_string().into(),
        message: source.kind.to_string(),
        related_information,
        ..Default::default()
    };

    result.push(main);

    for info in extra_info {
        result.push(lsp_types::Diagnostic {
            range: ts_range_to_lsp(&doc.rope, &info.range),
            source: SERVER_NAME.to_string().into(),
            message: info.message,
            severity: Some(DiagnosticSeverity::HINT),
            ..Default::default()
        });
    }

    result
}

fn get_related_info(
    doc: &Document,
    extra_info: &[DiagnosticRelatedInfo],
) -> Option<Vec<DiagnosticRelatedInformation>> {
    if extra_info.is_empty() {
        return None;
    }

    extra_info
        .iter()
        .map(|info| DiagnosticRelatedInformation {
            location: Location::new(doc.uri.clone(), ts_range_to_lsp(&doc.rope, &info.range)),
            message: info.message.clone(),
        })
        .collect::<Vec<_>>()
        .into()
}

fn convert_diagnostic_level(level: DiagnosticLevel) -> DiagnosticSeverity {
    match level {
        DiagnosticLevel::Error => DiagnosticSeverity::ERROR,
        DiagnosticLevel::Warning => DiagnosticSeverity::WARNING,
        DiagnosticLevel::Info => DiagnosticSeverity::INFORMATION,
        DiagnosticLevel::Help => DiagnosticSeverity::HINT,
    }
}

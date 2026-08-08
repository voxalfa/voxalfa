use crate::{
    Validator,
    diagnostics::types::{Diagnostic, DiagnosticLevel},
};

use codespan_reporting::{
    diagnostic::{self as codespan},
    files::SimpleFiles,
    term::{self, Config},
};

pub fn assert_diagnostic_snapshot(source_name: &str, content: &str) {
    let mut validator = Validator::init().unwrap();
    let output = validator.analyze(content);
    let diagnostics = output.diagnostics;

    if diagnostics.is_empty() {
        insta::assert_snapshot!(
            source_name,
            format!(
                "=== INPUT ===\n{}\n\n(No diagnostics reported)",
                content.trim()
            )
        );

        return;
    }

    let mut files = SimpleFiles::new();
    let mut output_str = String::new();

    let file_id = files.add(format!("{source_name}.vfa"), content);
    let config = Config::default();

    output_str.push_str("=== INPUT ===\n");
    output_str.push_str(content.trim());
    output_str.push_str("\n\n=== DIAGNOSTICS ===\n");

    for diagnostic in diagnostics {
        let result = build_diagnostic(file_id, diagnostic);
        let rendered = term::emit_into_string(&config, &files, &result).unwrap();

        output_str.push_str(&rendered);
    }

    insta::assert_snapshot!(source_name, output_str);
}

fn build_diagnostic(file_id: usize, diagnostic: Diagnostic) -> codespan::Diagnostic<usize> {
    let severity = match diagnostic.level {
        DiagnosticLevel::Error => codespan::Severity::Error,
        DiagnosticLevel::Warning => codespan::Severity::Warning,
        DiagnosticLevel::Info => codespan::Severity::Note,
        DiagnosticLevel::Help => codespan::Severity::Help,
    };

    let message = diagnostic.kind.to_string();
    let extra_info = diagnostic.kind.get_extra_info();
    let label = diagnostic.kind.get_label().unwrap_or_default();
    let range = diagnostic.byte_range();

    codespan::Diagnostic::new(severity)
        .with_code(diagnostic.kind.get_code())
        .with_message(&message)
        .with_label(codespan::Label::primary(file_id, range).with_message(label))
        .with_labels_iter(extra_info.into_iter().map(|info| {
            codespan::Label::secondary(file_id, info.range.start_byte..info.range.end_byte)
                .with_message(info.message)
        }))
}

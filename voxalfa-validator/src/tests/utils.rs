use crate::{
    MultiStepValidator,
    diagnostics::types::{Diagnostic, DiagnosticLevel},
};

pub fn assert_diagnostic_snapshot(source_name: &str, content: &str) {
    let mut validator = MultiStepValidator::init().unwrap();
    let output = validator.process(content);
    let diagnostics = output.diagnostics;

    if diagnostics.is_empty() {
        insta::assert_snapshot!(format!(
            "=== INPUT ===\n{}\n\n(No diagnostics reported)",
            content.trim()
        ));

        return;
    }

    let mut output = String::new();

    output.push_str("=== INPUT ===\n");
    output.push_str(content.trim());
    output.push_str("\n\n=== DIAGNOSTICS ===\n");

    for (i, diag) in diagnostics.iter().enumerate() {
        if i > 0 {
            output.push_str("\n");
        }
        output.push_str(&format_annotated_diagnostic(source_name, content, diag));
    }

    insta::assert_snapshot!(output);
}

fn format_annotated_diagnostic(source_name: &str, content: &str, diag: &Diagnostic) -> String {
    let mut output = String::new();

    let level_str = match diag.level {
        DiagnosticLevel::Error => "error",
        DiagnosticLevel::Warning => "warning",
        DiagnosticLevel::Info => "info",
        DiagnosticLevel::Help => "help",
    };

    output.push_str(&format!(
        "{}[{}]: {}\n",
        level_str,
        diag.kind.get_code(),
        diag.kind
    ));

    let primary_line_num = byte_to_line_num(content, diag.range.start_byte);

    output.push_str(&format!(" --> {source_name}.vfa:{}:1\n", primary_line_num));
    output.push_str("  |\n");

    let lines: Vec<&str> = content.lines().collect();

    for related in diag.kind.get_extra_info() {
        let line_num = byte_to_line_num(content, related.range.start_byte);
        if let Some(line_content) = lines.get(line_num - 1) {
            output.push_str(&format!("{:2} | {}\n", line_num, line_content));
            output.push_str(&format!(
                "   | {} {}\n",
                "-".repeat(line_content.len().max(1)),
                related.message
            ));
        }
    }

    if let Some(line_content) = lines.get(primary_line_num - 1) {
        output.push_str(&format!("{:2} | {}\n", primary_line_num, line_content));

        let label = diag
            .kind
            .get_label()
            .unwrap_or_else(|| level_str.to_string());

        output.push_str(&format!(
            "   | {} {}\n",
            "^".repeat(line_content.len().max(1)),
            label
        ));
    }

    output
}

fn byte_to_line_num(source: &str, byte_idx: usize) -> usize {
    source[..byte_idx.min(source.len())].lines().count().max(1)
}

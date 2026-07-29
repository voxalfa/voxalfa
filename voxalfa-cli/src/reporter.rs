use clap::builder::styling::*;
use codespan_reporting::{
    diagnostic::{self as codespan, Severity},
    files::SimpleFiles,
    term::{
        self,
        termcolor::{ColorChoice, StandardStream},
    },
};
use similar::{ChangeTag, TextDiff};
use voxalfa_validator::diagnostics::types::{Diagnostic, DiagnosticLevel};

use crate::types::SourceFile;

#[derive(Debug)]
struct FormattingDiff {
    path: String,
    original: String,
    formatted: String,
}

#[derive(Debug)]
pub struct CliReporter<'a> {
    config: term::Config,
    files: SimpleFiles<String, &'a str>,
    diagnostics: Vec<codespan::Diagnostic<usize>>,
    diffs: Vec<FormattingDiff>,
    file_count: usize,
}

impl<'a> CliReporter<'a> {
    pub fn new(file_count: usize) -> Self {
        Self {
            config: term::Config::default(),
            files: SimpleFiles::new(),
            diagnostics: Vec::new(),
            diffs: Vec::new(),
            file_count,
        }
    }

    pub fn register_diagnostics(&mut self, file: &'a SourceFile, diagnostics: Vec<Diagnostic>) {
        let file_id = self.files.add(file.path.to_string(), &file.content);

        for diagnostic in diagnostics {
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

            let result = codespan::Diagnostic::new(severity)
                .with_code(diagnostic.kind.get_code())
                .with_message(&message)
                .with_label(codespan::Label::primary(file_id, range).with_message(label))
                .with_labels_iter(extra_info.into_iter().map(|info| {
                    codespan::Label::secondary(file_id, info.range.start_byte..info.range.end_byte)
                        .with_message(info.message)
                }));

            self.diagnostics.push(result);
        }
    }

    pub fn register_diff(&mut self, path: String, original: String, formatted: String) {
        self.diffs.push(FormattingDiff {
            path,
            original,
            formatted,
        });
    }

    pub fn finalize(&mut self) {
        if !self.diagnostics.is_empty() {
            let writer = StandardStream::stderr(ColorChoice::Always);

            for diagnostic in &self.diagnostics {
                term::emit_to_write_style(
                    &mut writer.lock(),
                    &self.config,
                    &self.files,
                    diagnostic,
                )
                .unwrap();
            }
        }

        if !self.diffs.is_empty() {
            for diff in &self.diffs {
                self.print_diff(&diff.path, &diff.original, &diff.formatted);
            }
        }

        if self.has_errors_or_diffs(Severity::Help) {
            self.print_summary_report();
        }

        if self.has_errors_or_diffs(Severity::Error) {
            std::process::exit(1);
        }
    }

    pub fn print_error<D: std::fmt::Display>(message: D) {
        let red = Style::new()
            .fg_color(Some(AnsiColor::BrightRed.into()))
            .bold();

        eprintln!("{red}error{red:#}: {message}");
        std::process::exit(1);
    }

    fn has_errors_or_diffs(&self, min_severity: Severity) -> bool {
        self.diagnostics.iter().any(|d| d.severity >= min_severity) || !self.diffs.is_empty()
    }

    fn print_diff(&self, path: &str, original: &str, formatted: &str) {
        let diff = TextDiff::from_lines(original, formatted);

        let cyan = Style::new().fg_color(Some(AnsiColor::Cyan.into()));
        let red = Style::new().fg_color(Some(AnsiColor::Red.into()));
        let green = Style::new().fg_color(Some(AnsiColor::Green.into()));

        eprintln!("{cyan}    ┌─{cyan:#} {path}");
        eprintln!("{cyan}    │{cyan:#}");

        for group in diff.grouped_ops(0) {
            for op in group {
                for change in diff.iter_changes(&op) {
                    let line_no = change
                        .old_index()
                        .or_else(|| change.new_index())
                        .map(|i| i + 1)
                        .unwrap_or(1);

                    let line_content = change.value().trim_end();

                    match change.tag() {
                        ChangeTag::Delete => {
                            eprintln!("{cyan}{line_no:>3} │{cyan:#} {red}- {line_content}{red:#}");
                        }
                        ChangeTag::Insert => {
                            eprintln!("{cyan}    │{cyan:#} {green}+ {line_content}{green:#}");
                        }
                        ChangeTag::Equal => {}
                    }
                }
            }
        }

        eprintln!();
    }

    fn get_diagnostic_count(&self, severity: Severity) -> usize {
        self.diagnostics
            .iter()
            .filter(|d| d.severity == severity)
            .count()
    }

    fn print_summary_report(&self) {
        let bold = Style::new().bold();
        let error = Style::new()
            .fg_color(Some(AnsiColor::BrightRed.into()))
            .bold();
        let yellow = Style::new()
            .fg_color(Some(AnsiColor::BrightYellow.into()))
            .bold();

        let error_count = self.get_diagnostic_count(Severity::Error);
        let warning_count = self.get_diagnostic_count(Severity::Warning);
        let diff_count = self.diffs.len();

        if diff_count > 0 {
            let file_plural = if diff_count == 1 { "file" } else { "files" };

            eprintln!(
                "{error}error{error:#}: {diff_count} {file_plural} (out of {}) formatted improperly:",
                self.file_count
            );

            for diff in &self.diffs {
                eprintln!("  {bold}•{bold:#} {}", diff.path);
            }

            eprintln!("\nRun without {bold}--check{bold:#} to apply changes automatically.");
        }

        let mut diag_parts = Vec::new();

        if error_count > 0 {
            diag_parts.push(format!(
                "{error}{error_count} error{}{error:#}",
                if error_count == 1 { "" } else { "s" }
            ));
        }
        if warning_count > 0 {
            diag_parts.push(format!(
                "{yellow}{warning_count} warning{}{yellow:#}",
                if warning_count == 1 { "" } else { "s" }
            ));
        }

        if !diag_parts.is_empty() {
            if diff_count > 0 {
                eprintln!();
            }

            eprintln!(
                "{} emitted across {} {}",
                diag_parts.join(", "),
                self.file_count,
                if self.file_count == 1 {
                    "file"
                } else {
                    "files"
                }
            );
        }
    }
}

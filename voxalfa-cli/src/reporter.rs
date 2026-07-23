use codespan_reporting::{
    diagnostic as codespan,
    files::SimpleFiles,
    term::{
        self,
        termcolor::{ColorChoice, StandardStream},
    },
};

use voxalfa_validator::diagnostics::types::{Diagnostic, DiagnosticLevel};

use crate::types::SourceFile;

#[derive(Debug)]
pub struct CliReporter<'a> {
    config: term::Config,
    files: SimpleFiles<String, &'a str>,
    diagnostics: Vec<codespan::Diagnostic<usize>>,
}

impl Default for CliReporter<'_> {
    fn default() -> Self {
        Self {
            config: term::Config::default(),
            files: SimpleFiles::new(),
            diagnostics: Vec::new(),
        }
    }
}

impl<'a> CliReporter<'a> {
    pub fn register(&mut self, file: &'a SourceFile, diagnostics: Vec<Diagnostic>) {
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

    pub fn display_report(&mut self) {
        let writer = StandardStream::stderr(ColorChoice::Always);

        for diagnostic in &self.diagnostics {
            term::emit_to_write_style(&mut writer.lock(), &self.config, &self.files, diagnostic)
                .unwrap();
        }
    }
}

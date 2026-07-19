mod error;
mod reporter;

use std::fs::{self, File};

use clap::{Parser, Subcommand};
use voxalfa_formatter::Formatter;
use voxalfa_validator::{
    diagnostic::Diagnostic, output::ValidatorOutput, ts_utils::context::TSContext,
    validator::DocumentValidator,
};

use crate::{error::Error, reporter::CliReporter};

#[derive(Parser)]
pub struct Cli {
    #[command(subcommand)]
    command: CliCommand,
}

#[derive(Subcommand)]
pub enum CliCommand {
    Format { file: String },
    Check { file: String },
}

fn main() {
    let cli = Cli::parse();

    let res = match cli.command {
        CliCommand::Format { file } => format(&file),
        CliCommand::Check { file } => check(&file),
    };

    if let Err(err) = res {
        eprintln!("Error: {err}");
        std::process::exit(1);
    }
}

fn format(file_path: &str) -> Result<(), Error> {
    let (source, output) = parse_file(file_path)?;

    if output.diagnostics.iter().any(|d| d.is_error()) {
        show_diagnostics(file_path, &source, output.diagnostics);
    } else {
        let formatter = Formatter::default();
        let mut writter = File::create(file_path)?;

        formatter.format(&output, &mut writter)?;
    }

    Ok(())
}

fn check(file_path: &str) -> Result<(), Error> {
    let (source, output) = parse_file(file_path)?;

    show_diagnostics(file_path, &source, output.diagnostics);

    Ok(())
}

fn show_diagnostics(file_path: &str, content: &str, diagnostics: Vec<Diagnostic>) {
    let mut cli_reporter = CliReporter::default();

    cli_reporter.register(file_path, &content, diagnostics);
    cli_reporter.display_report();
}

fn parse_file(file_path: &str) -> Result<(String, ValidatorOutput), Error> {
    let mut ts_context = TSContext::new()?;

    let content = fs::read_to_string(file_path)?;
    let validator = DocumentValidator::new(&content);
    let output = validator.validate(&mut ts_context);

    Ok((content, output))
}

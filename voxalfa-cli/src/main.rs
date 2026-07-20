mod error;
mod reporter;
mod types;

use std::fs::{self, File};

use clap::{Parser, Subcommand};
use voxalfa_formatter::Formatter;
use voxalfa_validator::{
    output::ValidatorOutput, ts_utils::context::TSContext, validator::DocumentValidator,
};

use crate::{error::Error, reporter::CliReporter, types::SourceFile};

#[derive(Parser)]
#[clap(about, version)]
pub struct Cli {
    #[command(subcommand)]
    command: CliCommand,
}

#[derive(Subcommand)]
pub enum CliCommand {
    /// Format the provided partition files
    Format {
        /// Absolute file paths or patterns
        file: Vec<String>,
    },
    /// Format the provided partition files
    Check {
        /// Absolute file paths or patterns
        file: Vec<String>,
    },
}

fn main() {
    let cli = Cli::parse();

    let res = match cli.command {
        CliCommand::Format { file } => format(file),
        CliCommand::Check { file } => check(file),
    };

    if let Err(err) = res {
        eprintln!("Error: {err}");
        std::process::exit(1);
    }
}

fn format(file_paths: Vec<String>) -> Result<(), Error> {
    let mut cli_reporter = CliReporter::default();

    let files = read_files(file_paths)?;

    for file in &files {
        let output = parse_file(&file.content)?;

        if output.diagnostics.iter().any(|d| d.is_error()) {
            cli_reporter.register(&file, output.diagnostics);
        } else {
            let formatter = Formatter::new(&output);
            let mut writter = File::create(&file.path)?;

            formatter.format(&mut writter)?;
        }
    }

    cli_reporter.display_report();

    Ok(())
}

fn check(file_paths: Vec<String>) -> Result<(), Error> {
    let mut cli_reporter = CliReporter::default();

    let files = read_files(file_paths)?;

    for file in &files {
        let output = parse_file(&file.content)?;

        cli_reporter.register(&file, output.diagnostics);
    }

    cli_reporter.display_report();

    Ok(())
}

fn read_files(file_paths: Vec<String>) -> Result<Vec<SourceFile>, Error> {
    let mut results = Vec::new();

    for path in file_paths {
        let content = fs::read_to_string(&path)?;
        let file = SourceFile { path, content };

        results.push(file);
    }

    Ok(results)
}

fn parse_file(content: &str) -> Result<ValidatorOutput, Error> {
    let mut ts_context = TSContext::new()?;

    let validator = DocumentValidator::new(content);
    let output = validator.validate(&mut ts_context);

    Ok(output)
}

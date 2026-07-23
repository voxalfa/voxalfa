use clap::Parser;

use crate::{
    error::Error,
    reporter::CliReporter,
    utils::{parse_file, read_files},
};

#[derive(Parser)]
pub struct CheckParams {
    /// Absolute file paths or patterns
    file: Vec<String>,
}

pub fn execute(params: CheckParams) -> Result<(), Error> {
    let mut cli_reporter = CliReporter::default();

    let files = read_files(params.file)?;

    for file in &files {
        let output = parse_file(&file.content)?;

        cli_reporter.register(file, output.diagnostics);
    }

    cli_reporter.display_report();

    Ok(())
}

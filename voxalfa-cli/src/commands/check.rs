use clap::Parser;

use crate::{
    error::Result,
    reporter::CliReporter,
    utils::fs::{parse_file, read_files},
};

#[derive(Parser)]
pub struct CheckParams {
    /// Absolute file paths or patterns
    file: Vec<String>,
}

pub fn execute(params: CheckParams) -> Result<()> {
    let files = read_files(&params.file)?;
    let mut cli_reporter = CliReporter::new(files.len());

    for file in &files {
        let output = parse_file(&file.content)?;

        cli_reporter.register_diagnostics(file, output.diagnostics);
    }

    cli_reporter.finalize();

    Ok(())
}

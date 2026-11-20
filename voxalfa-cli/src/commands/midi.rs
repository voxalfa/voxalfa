use std::process::ExitCode;

use clap::Parser;
use voxalfa_midi::Converter;

use crate::{
    error::Result,
    reporter::CliReporter,
    utils::fs::{parse_file, read_files, resolve_output_path},
};

#[derive(Parser)]
pub struct MidiParams {
    /// Absolute file paths or patterns
    files: Vec<String>,
    #[clap(short, long)]
    output: Option<String>,
}

pub fn execute(params: MidiParams) -> Result<ExitCode> {
    let files = read_files(&params.files)?;
    let mut reporter = CliReporter::new(1);

    for file in &files {
        let output = parse_file(&file.content)?;

        if output.has_error() {
            reporter.register_diagnostics(file, output.diagnostics);
        } else {
            let converter = Converter::new(&output);
            let smf = converter.convert()?;
            let output_path = resolve_output_path(&file.path, params.output.as_deref(), "mid");

            smf.save(output_path)?;
        }
    }

    Ok(reporter.finalize())
}

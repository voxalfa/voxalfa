use std::process::ExitCode;

use clap::Parser;
use voxalfa_musicxml::Converter;

use crate::{
    error::Result,
    reporter::CliReporter,
    utils::fs::{parse_file, read_files, resolve_output_path, write_file},
};

#[derive(Parser)]
pub struct XmlParams {
    /// Absolute file paths or patterns
    files: Vec<String>,
    #[clap(short, long)]
    output: Option<String>,
}

pub fn execute(params: XmlParams) -> Result<ExitCode> {
    let files = read_files(&params.files)?;
    let mut reporter = CliReporter::new(1);

    for file in &files {
        let output = parse_file(&file.content)?;

        if output.has_error() {
            reporter.register_diagnostics(file, output.diagnostics);
        } else {
            let converter = Converter::new(&output);
            let xml = converter.convert().unwrap();
            let output_path = resolve_output_path(&file.path, params.output.as_deref(), "xml");

            write_file(output_path, xml)?;
        }
    }

    Ok(reporter.finalize())
}

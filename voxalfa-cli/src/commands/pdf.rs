use std::process::ExitCode;

use clap::Parser;
use voxalfa_svg::RenderOptions;

use crate::{
    error::Result,
    reporter::CliReporter,
    utils::fs::{parse_file, read_files, resolve_output_path, write_file},
};

#[derive(Parser)]
pub struct PdfParams {
    /// Absolute file paths or patterns
    files: Vec<String>,
    #[clap(short, long)]
    output: Option<String>,
}

pub fn execute(params: PdfParams) -> Result<ExitCode> {
    let files = read_files(&params.files)?;
    let mut reporter = CliReporter::new(1);

    for file in &files {
        let output = parse_file(&file.content)?;

        if output.has_error() {
            reporter.register_diagnostics(file, output.diagnostics);
        } else {
            let pdf = voxalfa_pdf::render(&output, &RenderOptions::default())?;
            let output_path = resolve_output_path(&file.path, params.output.as_deref(), "pdf");

            write_file(output_path, pdf)?;
        }
    }

    Ok(reporter.finalize())
}

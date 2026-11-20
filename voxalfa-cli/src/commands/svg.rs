use std::process::ExitCode;

use clap::Parser;
use voxalfa_svg::render_pages_with;

use crate::{
    error::Result,
    reporter::CliReporter,
    utils::fs::{parse_file, read_files, resolve_output_path, resolve_page_path, write_file},
};

#[derive(Parser)]
pub struct SvgParams {
    /// Absolute file paths or patterns
    files: Vec<String>,
    #[clap(short, long)]
    output: Option<String>,
}

pub fn execute(params: SvgParams) -> Result<ExitCode> {
    let files = read_files(&params.files)?;
    let mut reporter = CliReporter::new(1);

    for file in &files {
        let output = parse_file(&file.content)?;

        if output.has_error() {
            reporter.register_diagnostics(file, output.diagnostics);
        } else {
            let pages = render_pages_with(&output, Default::default())?;
            let base_path = resolve_output_path(&file.path, params.output.as_deref(), "svg");

            for (index, page) in pages.iter().enumerate() {
                let output_path = resolve_page_path(&base_path, index, pages.len());

                write_file(output_path, page)?;
            }
        }
    }

    Ok(reporter.finalize())
}

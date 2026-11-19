use std::path::{Path, PathBuf};

use clap::Parser;
use voxalfa_musicxml::Converter;

use crate::{
    error::Result,
    reporter::CliReporter,
    utils::fs::{parse_file, read_files, write_file},
};

#[derive(Parser)]
pub struct XmlParams {
    /// Absolute file paths or patterns
    files: Vec<String>,
    #[clap(short, long)]
    output: Option<String>,
}

pub fn execute(params: XmlParams) -> Result<()> {
    let files = read_files(&params.files)?;
    let mut reporter = CliReporter::new(1);

    for file in &files {
        let output = parse_file(&file.content)?;

        if output.has_error() {
            reporter.register_diagnostics(file, output.diagnostics);
        } else {
            let converter = Converter::new(&output);
            let xml = converter.convert().unwrap();
            let file_path = Path::new(&file.path);
            let output_path = params.output.as_ref().map(PathBuf::from);
            let default_path = file_path.with_extension("xml");
            let base_path = default_path.file_name();

            let output_path = match output_path {
                Some(path) if path.extension().is_some() => path,
                Some(path) if let Some(base) = base_path => path.join(base),
                _ => default_path,
            };

            write_file(output_path, xml)?;
        }
    }

    reporter.finalize();

    Ok(())
}

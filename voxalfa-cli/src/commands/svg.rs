use std::{
    fs::File,
    io::Write,
    path::{Path, PathBuf},
};
use voxalfa_svg::renderer::Renderer;

use clap::Parser;

use crate::{
    error::Result,
    reporter::CliReporter,
    utils::fs::{parse_file, read_files},
};

#[derive(Parser)]
pub struct SvgParams {
    /// Absolute file paths or patterns
    files: Vec<String>,
    #[clap(short, long)]
    output: Option<String>,
}

pub fn execute(params: SvgParams) -> Result<()> {
    let files = read_files(&params.files)?;
    let mut reporter = CliReporter::new(1);

    for file in &files {
        let output = parse_file(&file.content)?;

        if output.has_error() {
            reporter.register_diagnostics(file, output.diagnostics);
        } else {
            let converter = Renderer::new(output)?;
            let svg = converter.render_to_svg()?;
            let file_path = Path::new(&file.path);
            let output_path = params.output.as_ref().map(PathBuf::from);
            let default_path = file_path.with_extension("svg");
            let base_path = default_path.file_name();

            let output_path = match output_path {
                Some(path) if path.is_file() => path,
                Some(path) if let Some(base) = base_path => path.join(base),
                _ => default_path,
            };

            File::options()
                .create(true)
                .write(true)
                .truncate(true)
                .open(output_path)?
                .write_all(svg.as_bytes())?;
        }
    }

    reporter.finalize();

    Ok(())
}

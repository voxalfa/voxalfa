use std::path::{Path, PathBuf};

use clap::Parser;
use voxalfa_midi::Converter;

use crate::{
    error::Result,
    reporter::CliReporter,
    utils::{parse_file, read_files},
};

#[derive(Parser)]
pub struct MidiParams {
    /// Absolute file paths or patterns
    files: Vec<String>,
    #[clap(short, long)]
    output: Option<String>,
}

pub fn execute(params: MidiParams) -> Result<()> {
    let files = read_files(&params.files)?;
    let mut reporter = CliReporter::new(1);

    for file in &files {
        let output = parse_file(&file.content)?;

        if output.has_error() {
            reporter.register_diagnostics(file, output.diagnostics);
        } else {
            let converter = Converter::new(&output);
            let smf = converter.convert()?;
            let file_path = Path::new(&file.path);
            let output_path = params.output.as_ref().map(PathBuf::from);
            let default_path = file_path.with_extension("mid");
            let base_path = default_path.file_name();

            let output_path = match output_path {
                Some(path) if path.is_file() => path,
                Some(path) if let Some(base) = base_path => path.join(base),
                _ => default_path,
            };

            smf.save(output_path)?;
        }
    }

    reporter.finalize();

    Ok(())
}

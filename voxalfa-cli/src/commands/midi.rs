use std::path::Path;

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
    // #[clap(short, long)]
    // output: Option<String>,
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
            let output_path = Path::new(&file.path).with_extension("mid");
            let smf = converter.convert()?;

            smf.save(output_path)?;
        }
    }

    reporter.finalize();

    Ok(())
}

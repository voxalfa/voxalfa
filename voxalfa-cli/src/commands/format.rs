use std::fs::File;

use clap::Parser;
use voxalfa_formatter::Formatter;

use crate::{
    error::Error,
    reporter::CliReporter,
    utils::{parse_file, read_files},
};

#[derive(Parser)]
pub struct FormatParams {
    /// Absolute file paths or patterns
    file: Vec<String>,
    /// Compare the file to the formatted version
    #[clap(short, long)]
    check: bool,
}

pub fn execute(params: FormatParams) -> Result<(), Error> {
    let files = read_files(params.file)?;
    let mut cli_reporter = CliReporter::new(files.len());

    for file in &files {
        let output = parse_file(&file.content)?;

        // TODO: recoverable error implementation
        if output.diagnostics.iter().any(|d| d.is_error()) {
            cli_reporter.register_diagnostics(file, output.diagnostics);
        } else {
            let formatter = Formatter::new(&output);

            if params.check {
                let mut buffer = Vec::new();
                formatter.format(&mut buffer)?;
                let expected = String::from_utf8_lossy(&buffer);

                if expected != file.content {
                    cli_reporter.register_diff(
                        file.path.clone(),
                        file.content.clone(),
                        expected.into_owned(),
                    );
                }
            } else {
                let mut writer = File::create(&file.path)?;
                formatter.format(&mut writer)?;
            }
        }
    }

    cli_reporter.finalize();

    Ok(())
}

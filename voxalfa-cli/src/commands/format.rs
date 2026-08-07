use std::fs::File;

use clap::Parser;
use voxalfa_formatter::Formatter;

use crate::{
    error::Result,
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

pub fn execute(params: FormatParams) -> Result<()> {
    let files = read_files(&params.file)?;
    let mut cli_reporter = CliReporter::new(files.len());

    for file in &files {
        let output = parse_file(&file.content)?;

        if output.has_syntax_error() {
            cli_reporter.register_diagnostics(file, output.diagnostics);
        } else {
            let formatter = Formatter::new(&output);

            if params.check {
                let expected = formatter.format_to_string()?;

                if expected != file.content {
                    cli_reporter.register_diff(file.path.clone(), file.content.clone(), expected);
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

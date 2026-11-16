use std::io::{self, Read};

use clap::Parser;
use voxalfa_formatter::Formatter;
use voxalfa_linker::{Linker, TokenizerOutput};

use crate::{
    error::Result,
    reporter::CliReporter,
    utils::fs::{parse_file, read_file, write_file},
};

#[derive(Parser)]
pub struct LinkParams {
    /// File path
    file: String,
    /// Tokenized lyrics in JSON format (reads from stdin when omitted)
    #[clap(short, long)]
    tokens: Option<String>,
    /// Only annotate lyric slots, without linking
    #[clap(short, long)]
    annotate: bool,
}

pub fn execute(params: LinkParams) -> Result<()> {
    let file = read_file(params.file.into())?;
    let output = parse_file(&file.content)?;
    let mut cli_reporter = CliReporter::new(1);

    if output.has_syntax_error() {
        cli_reporter.register_diagnostics(&file, output.diagnostics);
        cli_reporter.finalize();

        return Ok(());
    }

    let content = Formatter::new(&output).annotate()?;

    if params.annotate {
        write_file(&file.path, content)?;
        cli_reporter.finalize();

        return Ok(());
    }

    let json = match params.tokens {
        Some(path) => read_file(path.into())?.content,
        None => read_stdin()?,
    };

    let tokenizer_output = TokenizerOutput::from_json(&json)?;

    match Linker::new(&content, &tokenizer_output).link() {
        Ok(linked) => {
            write_file(&file.path, linked)?;
        }
        Err(error) => {
            if let Some(partial) = error.partial() {
                print!("{partial}");
            }

            cli_reporter.finalize();

            return Err(error.into());
        }
    }

    cli_reporter.finalize();

    Ok(())
}

fn read_stdin() -> Result<String> {
    let mut buffer = String::new();

    io::stdin().read_to_string(&mut buffer)?;

    Ok(buffer)
}

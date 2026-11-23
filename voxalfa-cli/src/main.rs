mod commands;
mod error;
mod reporter;
mod types;
mod utils;

use std::process::ExitCode;

use clap::Parser;

use crate::{commands::*, reporter::CliReporter};

#[derive(Parser)]
#[clap(
    about,
    version = voxalfa_build_info::VERSION,
    long_version = voxalfa_build_info::long_version()
)]
pub struct Cli {
    #[command(subcommand)]
    command: CliCommand,
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    let result = match cli.command {
        CliCommand::Format(params) => format::execute(params),
        CliCommand::Check(params) => check::execute(params),
        CliCommand::Link(params) => link::execute(params),
        CliCommand::Midi(params) => midi::execute(params),
        CliCommand::Xml(params) => xml::execute(params),
        CliCommand::Svg(params) => svg::execute(params),
        CliCommand::Pdf(params) => pdf::execute(params),
        CliCommand::Lyrics(params) => lyrics::execute(params),
    };

    match result {
        Ok(code) => code,
        Err(error) => {
            CliReporter::print_error(error);
            ExitCode::FAILURE
        }
    }
}

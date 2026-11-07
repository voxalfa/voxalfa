mod commands;
mod error;
mod reporter;
mod types;
mod utils;

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

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        CliCommand::Format(params) => format::execute(params),
        CliCommand::Check(params) => check::execute(params),
        CliCommand::Midi(params) => midi::execute(params),
        CliCommand::Svg(params) => svg::execute(params),
        CliCommand::Pdf(params) => pdf::execute(params),
        CliCommand::Lyrics(params) => lyrics::execute(params),
    };

    if let Err(error) = result {
        CliReporter::print_error(error);
    }
}

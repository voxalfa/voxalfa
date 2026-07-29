mod commands;
mod error;
mod reporter;
mod types;
mod utils;

use clap::Parser;

use crate::commands::*;

#[derive(Parser)]
#[clap(about, version)]
pub struct Cli {
    #[command(subcommand)]
    command: CliCommand,
}

fn main() {
    let cli = Cli::parse();

    let res = match cli.command {
        CliCommand::Format(params) => format::execute(params),
        CliCommand::Check(params) => check::execute(params),
        CliCommand::Midi(params) => midi::execute(params),
    };

    if let Err(err) = res {
        eprintln!("Error: {err}");
        std::process::exit(1);
    }
}

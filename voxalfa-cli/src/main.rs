mod commands;
mod error;
mod reporter;
mod types;
mod utils;

use clap::Parser;

use crate::commands::CliCommand;

#[derive(Parser)]
#[clap(about, version)]
pub struct Cli {
    #[command(subcommand)]
    command: CliCommand,
}

fn main() {
    let cli = Cli::parse();

    let res = match cli.command {
        CliCommand::Format(params) => commands::format::execute(params),
        CliCommand::Check(params) => commands::check::execute(params),
    };

    if let Err(err) = res {
        eprintln!("Error: {err}");
        std::process::exit(1);
    }
}

mod commands;
mod error;
mod reporter;
mod types;
mod utils;

use clap::{Parser, Subcommand};

use crate::commands::{check::CheckParams, format::FormatParams};

#[derive(Parser)]
#[clap(about, version)]
pub struct Cli {
    #[command(subcommand)]
    command: CliCommand,
}

#[derive(Subcommand)]
pub enum CliCommand {
    /// Format the provided partition files
    Format(FormatParams),
    /// Format the provided partition files
    Check(CheckParams),
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

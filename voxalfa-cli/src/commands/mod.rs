pub mod check;
pub mod format;

use clap::Subcommand;

use crate::commands::{check::CheckParams, format::FormatParams};

#[derive(Subcommand)]
pub enum CliCommand {
    /// Format the provided partition files
    Format(FormatParams),
    /// Format the provided partition files
    Check(CheckParams),
}

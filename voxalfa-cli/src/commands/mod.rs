pub mod check;
pub mod format;
pub mod lyrics;
pub mod midi;

use clap::Subcommand;

use crate::commands::{
    check::CheckParams, format::FormatParams, lyrics::LyricsParams, midi::MidiParams,
};

#[derive(Subcommand)]
pub enum CliCommand {
    /// Format the provided partition files
    Format(FormatParams),
    /// Format the provided partition files
    Check(CheckParams),
    /// Convert partitions into MIDI files
    Midi(MidiParams),
    /// Manipulate lyrics from partition files
    Lyrics(LyricsParams),
}

pub mod check;
pub mod format;
pub mod lyrics;
pub mod midi;
pub mod svg;

use clap::Subcommand;

use crate::commands::{
    check::CheckParams, format::FormatParams, lyrics::LyricsParams, midi::MidiParams,
    svg::SvgParams,
};

#[derive(Subcommand)]
pub enum CliCommand {
    /// Format the provided partition files
    Format(FormatParams),
    /// Format the provided partition files
    Check(CheckParams),
    /// Convert partitions into MIDI files
    Midi(MidiParams),
    /// Convert partitions into SVG files
    Svg(SvgParams),
    /// Manipulate lyrics from partition files
    Lyrics(LyricsParams),
}

pub mod check;
pub mod format;
pub mod lyrics;
pub mod midi;
pub mod svg;
pub mod svg2;

use clap::Subcommand;

use crate::commands::{
    check::CheckParams, format::FormatParams, lyrics::LyricsParams, midi::MidiParams,
    svg::SvgParams, svg2::Svg2Params,
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
    /// Convert partitions into SVG files using the second-generation renderer
    Svg2(Svg2Params),
    /// Manipulate lyrics from partition files
    Lyrics(LyricsParams),
}

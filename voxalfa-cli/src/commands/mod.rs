pub mod check;
pub mod format;
pub mod lyrics;
pub mod midi;
pub mod pdf;
pub mod svg;

use clap::Subcommand;

use crate::commands::{
    check::CheckParams, format::FormatParams, lyrics::LyricsParams, midi::MidiParams,
    pdf::PdfParams, svg::SvgParams,
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
    /// Convert partitions into PDF files
    Pdf(PdfParams),
    /// Manipulate lyrics from partition files
    Lyrics(LyricsParams),
}

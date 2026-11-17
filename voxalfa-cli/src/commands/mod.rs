pub mod check;
pub mod format;
pub mod link;
pub mod lyrics;
pub mod midi;
pub mod pdf;
pub mod svg;
pub mod xml;

use clap::Subcommand;

use crate::commands::{
    check::CheckParams, format::FormatParams, link::LinkParams, lyrics::LyricsParams,
    midi::MidiParams, pdf::PdfParams, svg::SvgParams, xml::XmlParams,
};

#[derive(Subcommand)]
pub enum CliCommand {
    /// Format the provided partition files
    Format(FormatParams),
    /// Format the provided partition files
    Check(CheckParams),
    /// Link tokenized lyrics to the a given file
    Link(LinkParams),
    /// Convert partitions into MIDI files
    Midi(MidiParams),
    /// Convert partitions into MusicXML files
    Xml(XmlParams),
    /// Convert partitions into SVG files
    Svg(SvgParams),
    /// Convert partitions into PDF files
    Pdf(PdfParams),
    /// Manipulate lyrics from partition files
    Lyrics(LyricsParams),
}

use std::process::ExitCode;

use clap::Parser;
use voxalfa_core::{
    data_types::Voice,
    output::{
        lyrics::{LyricsBuilder, LyricsResolver},
        metrics::DummyMeasurer,
    },
};

use crate::{
    error::{Error, Result},
    reporter::CliReporter,
    utils::{
        fs::{parse_file, read_file},
        lyrics::CliVisitor,
    },
};

#[derive(Parser)]
pub struct LyricsParams {
    /// Absolute file paths or patterns
    file: String,
    /// Voice identifier with an optional 1-based index (e.g: S, T2, ...)
    #[clap(short, long)]
    voice: String,
}

pub fn execute(params: LyricsParams) -> Result<ExitCode> {
    let mut cli_reporter = CliReporter::new(1);

    let file = read_file(params.file.into())?;
    let output = parse_file(&file.content)?;

    let voice_id = parse_voice(&params.voice)
        .and_then(|(v, i)| output.symbols.resolve_voice_id(v, i))
        .ok_or(Error::InvalidVoice(params.voice))?;

    if output.has_error() {
        cli_reporter.register_diagnostics(&file, output.diagnostics);
    } else {
        let measurer = DummyMeasurer {};
        let builder = LyricsBuilder::new(measurer);
        let (_, lyrics_map) = builder.build_map::<CliVisitor>(&output, 0);
        let evaluator = LyricsResolver::new(voice_id, lyrics_map);
        let lyrics = evaluator.process::<CliVisitor>(&output);

        for (index, verse) in lyrics.iter().enumerate() {
            println!("{}. {verse}\n", index + 1);
        }
    }

    Ok(cli_reporter.finalize())
}

fn parse_voice(s: &str) -> Option<(Voice, usize)> {
    let base = s.chars().next()?;
    let voice = Voice::try_from(base.to_string()).ok()?;

    let index = s
        .get(1..)
        .filter(|s| !s.is_empty())
        .map(|i| i.parse())
        .transpose()
        .ok()?
        .unwrap_or(1);

    Some((voice, index))
}

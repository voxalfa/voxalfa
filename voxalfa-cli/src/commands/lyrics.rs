use clap::Parser;
use voxalfa_core::{
    data_types::Voice,
    output::{
        lyrics::{LyricsBuilder, LyricsEvaluator},
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
    /// Compare the file to the formatted version
    #[clap(short, long)]
    voice: String,
}

pub fn execute(params: LyricsParams) -> Result<()> {
    let mut cli_reporter = CliReporter::new(1);
    let voice =
        Voice::try_from(params.voice.clone()).map_err(|_| Error::InvalidVoice(params.voice))?;

    let file = read_file(params.file.into())?;
    let output = parse_file(&file.content)?;

    if output.has_error() {
        cli_reporter.register_diagnostics(&file, output.diagnostics);
    } else {
        let measurer = DummyMeasurer {};
        let builder = LyricsBuilder::new(measurer);
        let (_, lyrics_map) = builder.build_map::<CliVisitor>(&output, 0);
        let evaluator = LyricsEvaluator::new(lyrics_map);
        let voice_line = &output.build_voice_line(voice);
        let lyrics = evaluator.process(voice_line);

        for (index, verse) in lyrics.iter().enumerate() {
            println!("{}. {verse}\n", index + 1);
        }
    }

    cli_reporter.finalize();

    Ok(())
}

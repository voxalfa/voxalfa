use clap::{Parser, builder::styling::Style};
use voxalfa_validator::data_types::Voice;

use crate::{
    error::{Error, Result},
    reporter::CliReporter,
    utils::{parse_file, read_file},
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
        let underline = Style::new().underline();
        let ulhs = format!("{underline}");
        let urhs = format!("{underline:#}");
        let lyrics = output.build_lyrics(voice, &ulhs, &urhs);

        for (index, verse) in lyrics.iter().enumerate() {
            println!("{}. {verse}\n", index + 1);
        }
    }

    cli_reporter.finalize();

    Ok(())
}

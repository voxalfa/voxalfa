use async_lsp::lsp_types::TextEdit;
use voxalfa_validator::{data_types::Voice, output::FinalOutput, ts_utils::range::Position};

use crate::utils::ts_range_to_lsp;

// currently only for voices
pub fn resolve_rename_edits(
    new_name: String,
    position: Position,
    data: &FinalOutput,
) -> Option<Vec<TextEdit>> {
    let voice = Voice::try_from(new_name).ok()?;
    let refs = data.symbols.find_voice_refs(&position)?;

    let edits = refs
        .iter()
        .map(|&sid| data.symbols.get_symbol(sid))
        .map(|s| TextEdit {
            range: ts_range_to_lsp(&s.range),
            new_text: voice.to_string(),
        })
        .collect();

    Some(edits)
}

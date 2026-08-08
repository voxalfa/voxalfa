use async_lsp::lsp_types::{Position, TextEdit};
use voxalfa_core::data_types::Voice;

use crate::{
    state::Document,
    utils::{lsp_pos_to_ts, ts_range_to_lsp},
};

// currently only for voices
pub fn resolve_rename_edits(
    new_name: String,
    position: Position,
    doc: &Document,
) -> Option<Vec<TextEdit>> {
    let voice = Voice::try_from(new_name).ok()?;
    let position = lsp_pos_to_ts(&doc.rope, position);
    let refs = doc.data.symbols.find_voice_refs(&position)?;

    let edits = refs
        .iter()
        .map(|&sid| doc.data.symbols.get_symbol(sid))
        .map(|s| TextEdit {
            range: ts_range_to_lsp(&doc.rope, &s.range),
            new_text: voice.to_string(),
        })
        .collect();

    Some(edits)
}

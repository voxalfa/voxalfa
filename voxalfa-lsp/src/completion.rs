use async_lsp::lsp_types::{CompletionItem, CompletionItemKind, InsertTextFormat};

use voxalfa_validator::data_types::TimeSignature;

pub fn build_measure_snippet(ts: TimeSignature) -> CompletionItem {
    let mut snippet = String::from("[${1:v}] |");

    for pos in 0..ts.top as usize {
        let tab_stop = pos + 2;

        snippet.push_str(&format!("${{{tab_stop}:notes}}"));

        if pos < (ts.top as usize - 1) {
            let next_accent = ts.get_accent(pos + 1);
            let sep = format!(" {next_accent}");
            snippet.push_str(&sep);
        }
    }

    snippet.push_str(" ||");

    let label = format!("{}/{} Measure", ts.top, ts.bottom);

    CompletionItem {
        label: label.clone(),
        kind: Some(CompletionItemKind::SNIPPET),
        detail: Some(format!("Insert {label} measure pattern")),
        insert_text_format: Some(InsertTextFormat::SNIPPET),
        insert_text: Some(snippet),
        ..Default::default()
    }
}

pub fn voice_measure_snippets() -> Vec<CompletionItem> {
    vec![
        build_measure_snippet(TimeSignature { top: 2, bottom: 4 }),
        build_measure_snippet(TimeSignature { top: 3, bottom: 4 }),
        build_measure_snippet(TimeSignature { top: 4, bottom: 4 }),
        build_measure_snippet(TimeSignature { top: 6, bottom: 8 }),
    ]
}

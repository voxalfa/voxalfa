use async_lsp::lsp_types::{CompletionItem, CompletionItemKind, InsertTextFormat};
use tree_sitter::{Node, Point};
use voxalfa_validator::{
    data_types::{TimeSignature, Voice},
    ts_utils::generated::node_types,
};

use crate::state::Document;

pub fn get_completion_context(
    document: &Document,
    line: usize,
    column: usize,
) -> Option<CompletionContext> {
    let root = document.data.tree.as_ref().map(|t| t.root_node())?;
    let point = Point::new(line, column);
    let target = root.named_descendant_for_point_range(point, point)?;
    let line_str = document.source.lines().nth(line)?;

    match target.kind_id() {
        node_types::HEADER => {
            if line_str.contains("[$]") {
                Some(CompletionContext::InitialParams)
            } else if line_str.contains("[#]") {
                Some(CompletionContext::Metadata)
            } else {
                Some(CompletionContext::Header)
            }
        }
        node_types::SUB_SECTION => {
            if line_str.contains("[$]") {
                Some(CompletionContext::SectionParams)
            } else {
                let section = get_section_context(document, line)?;
                let context = CompletionContext::Section(section);

                Some(context)
            }
        }
        node_types::BUILTIN => {
            let builtin = get_builtin_context(target, &document.source)?;
            let context = CompletionContext::Builtin(builtin);

            Some(context)
        }
        _ => None,
    }
}

#[derive(Debug)]
pub enum CompletionContext {
    Header,
    Metadata,
    InitialParams,
    SectionParams,
    Section(SectionContext),
    Builtin(Builtin),
}

impl CompletionContext {
    pub fn completion_items(&self) -> Vec<CompletionItem> {
        match self {
            CompletionContext::Header => vec![
                self.snippet_item("parameters ($)", "Initial Parameters", "[\\$] ${0}"),
                self.snippet_item("metadata (#)", "Header metadata", "[#] ${0}"),
            ],
            CompletionContext::Metadata => self.header_metadata_snippets(),
            CompletionContext::InitialParams => self.initial_params_snippets(),
            CompletionContext::SectionParams => self.section_params_snippets(),
            CompletionContext::Section(context) => {
                vec![
                    context.build_section_snippet(1, "1 measure"),
                    context.build_section_snippet(2, "2 measures"),
                    context.build_section_snippet(3, "3 measures"),
                    self.snippet_item("verse 1", "Verse 1", "[1] ${0}"),
                    self.snippet_item("verse 2", "Verse 2", "[2] ${0}"),
                    self.snippet_item("verse 3", "Verse 3", "[3] ${0}"),
                    self.snippet_item("parameters ($)", "Section Parameters", "[\\$] ${0}"),
                ]
            }
            CompletionContext::Builtin(builtin) => builtin.completion_items(),
        }
    }

    pub fn header_metadata_snippets(&self) -> Vec<CompletionItem> {
        let properties = [
            ("title", "string", "title=\"${1:value}\""),
            ("author", "{string...}", "author={\"${1:value}\"}"),
            ("composer", "{string...}", "composer={\"${1:value}\"}"),
            ("verses", "integer", "verses={${1:1}}"),
            ("meter", "{number...}", "meter={${1:4}}"),
            ("description", "string", "description=\"${1:value}\""),
            ("release", "integer", "release={${1:2026}}"),
            ("language", "string", "language=\"${1:en}\""),
            ("tags", "{string...}", "tags={\"${1:tag}\"}"),
        ];

        self.build_param_snippets(&properties, "Header metadata field")
    }

    pub fn initial_params_snippets(&self) -> Vec<CompletionItem> {
        let properties = [
            ("key", "key", "key={${1:C}}"),
            ("time", "{integer,integer}", "time={${1:4},${2:4}}"),
            ("tempo", "tempo | integer", "tempo={${1:allegro}}"),
            ("voices", "{voice...}", "voices={${1:S}}"),
        ];

        self.build_param_snippets(&properties, "Initial parameter")
    }

    pub fn section_params_snippets(&self) -> Vec<CompletionItem> {
        let properties = [
            ("time", "{integer,integer}", "time={${1:4},${2:4}}"),
            ("tempo", "tempo | integer", "tempo={${1:Allegro}}"),
            ("label", "string", "label=\"${1:Section}\""),
            ("ending", "integer", "ending={${1:1}}"),
            ("key", "key", "key={${1:C}}"),
            ("jump", "jump", "jump={${1:DS}}"),
            ("mark", "mark", "mark={${1:S}}"),
            ("dynamics", "dynamic", "dynamics={${1:f}}"),
            ("touches", "{touch...}", "touches={${1:stc}}"),
            ("repeat", "integer", "repeat={${1:2}}"),
        ];

        self.build_param_snippets(&properties, "Section parameter")
    }

    fn build_param_snippets(
        &self,
        properties: &[(&str, &str, &str)],
        doc_label: &str,
    ) -> Vec<CompletionItem> {
        properties
            .iter()
            .map(|&(name, type_str, snippet)| CompletionItem {
                label: name.to_string(),
                kind: Some(CompletionItemKind::PROPERTY),
                detail: Some(format!("{name}: {type_str}")),
                documentation: Some(async_lsp::lsp_types::Documentation::String(
                    doc_label.to_string(),
                )),
                insert_text_format: Some(InsertTextFormat::SNIPPET),
                insert_text: Some(snippet.to_string()),
                ..Default::default()
            })
            .collect()
    }

    fn snippet_item(&self, label: &str, detail: &str, snippet: &str) -> CompletionItem {
        CompletionItem {
            label: label.to_string(),
            kind: Some(CompletionItemKind::SNIPPET),
            detail: Some(detail.to_string()),
            insert_text: Some(snippet.to_string()),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            ..Default::default()
        }
    }
}

#[derive(Debug)]
pub enum Builtin {
    Key,
    Voice,
    Tempo,
    Dynamics,
    Mark,
    Jump,
    Touches,
}

impl Builtin {
    fn completion_items(&self) -> Vec<CompletionItem> {
        match self {
            Builtin::Voice => [
                ("S", "Soprano"),
                ("A", "Alto"),
                ("T", "Tenor"),
                ("B", "Bass"),
            ]
            .iter()
            .map(|(v, d)| self.value_item(v, Some(d)))
            .collect(),
            Builtin::Key => [
                "C", "G", "D", "A", "E", "B", "F#", "F", "Bb", "Eb", "Ab", "Db", "Gb",
            ]
            .iter()
            .map(|k| self.value_item(k, None))
            .collect(),
            Builtin::Tempo => [
                ("grave", "Very slow and solemn"),
                ("largo", "Slow and broad"),
                ("adagio", "Slow and stately"),
                ("andante", "At a walking pace"),
                ("moderato", "At a moderate speed"),
                ("allegro", "Fast, quickly, and bright"),
                ("vivace", "Lively and fast"),
                ("presto", "Very fast"),
            ]
            .iter()
            .map(|(label, detail)| self.value_item(label, Some(detail)))
            .collect(),
            Builtin::Mark => ["S", "C", "TC", "F"]
                .iter()
                .map(|m| self.value_item(m, None))
                .collect(),
            Builtin::Touches => [("stc", "Staccato"), ("acc", "Accent"), ("frm", "Fermata")]
                .iter()
                .map(|(label, detail)| self.value_item(label, Some(detail)))
                .collect(),
            Builtin::Dynamics => ["ppp", "pp", "p", "mp", "mf", "f", "ff", "fff", "sfz"]
                .iter()
                .map(|d| self.value_item(d, None))
                .collect(),
            Builtin::Jump => [
                ("DS", "Dal Segno"),
                ("DC", "Da Capo"),
                ("DSC", "Dal Segno al Coda"),
                ("DSF", "Dal Segno al Fine"),
                ("DCC", "Da Capo al Coda"),
                ("DCF", "Da Capo al Fine"),
            ]
            .iter()
            .map(|(label, detail)| self.value_item(label, Some(detail)))
            .collect(),
        }
    }

    fn value_item(&self, label: &str, detail: Option<&str>) -> CompletionItem {
        CompletionItem {
            label: label.to_string(),
            kind: Some(CompletionItemKind::ENUM_MEMBER),
            detail: detail.map(|d| d.to_string()),
            insert_text: Some(label.to_string()),
            ..Default::default()
        }
    }
}

#[derive(Debug)]
pub struct SectionContext {
    voices: Vec<Voice>,
    time: TimeSignature,
    verses: u8,
}

impl SectionContext {
    fn build_section_snippet(&self, measures: usize, label_suffix: &str) -> CompletionItem {
        let mut snippet = String::new();
        let mut tab_stop = 1;

        let default_notes = ["s", "m", "d", "d"];

        for (v_idx, voice) in self.voices.iter().enumerate() {
            let note = default_notes.get(v_idx).unwrap_or(&"d");

            snippet.push_str(&format!("[{voice:?}] |"));

            for m in 0..measures {
                for pos in 0..self.time.top as usize {
                    snippet.push_str(&format!("${{{tab_stop}:{note}}}"));
                    tab_stop += 1;

                    if pos < (self.time.top as usize - 1) {
                        let next_accent = self.time.get_accent(pos + 1);
                        snippet.push_str(&format!(" {next_accent}"));
                    }
                }

                if m < measures - 1 {
                    snippet.push_str(" | ");
                } else {
                    snippet.push_str(" ||\n");
                }
            }
        }

        snippet.push('\n');

        for verse in 0..self.verses {
            snippet.push_str(&format!("[{}] ${{{tab_stop}:~}}\n", verse + 1));
            tab_stop += 1;
        }

        let voice_labels: Vec<String> = self.voices.iter().map(|v| format!("{v:?}")).collect();

        CompletionItem {
            label: format!("section ({label_suffix}, {})", voice_labels.join("")),
            kind: Some(CompletionItemKind::SNIPPET),
            detail: Some(format!(
                "Insert {} pattern for {} voice(s), {} verse(s) in {}/{}",
                label_suffix,
                self.voices.len(),
                self.verses,
                self.time.top,
                self.time.bottom
            )),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            insert_text: Some(snippet),
            ..Default::default()
        }
    }
}

fn get_builtin_context(target: Node<'_>, source: &str) -> Option<Builtin> {
    let identifier = target.prev_named_sibling()?;
    let source = source.as_bytes();
    let identifier = identifier.utf8_text(source).ok()?;

    match identifier {
        "key" => Some(Builtin::Key),
        "voices" => Some(Builtin::Voice),
        "tempo" => Some(Builtin::Tempo),
        "mark" => Some(Builtin::Mark),
        "jump" => Some(Builtin::Jump),
        "dynamics" => Some(Builtin::Dynamics),
        "touches" => Some(Builtin::Touches),
        _ => None,
    }
}

fn get_section_context(document: &Document, line: usize) -> Option<SectionContext> {
    let header = &document.data.header;
    let voices = header.params.voices.clone()?.value;
    let verses = header.metadata.verses.clone().map(|v| v.value);
    let mut time = header.params.time.clone()?.value;

    for section in &document.data.ir.sections {
        let range = document.data.symbols.get_scope_range(section.sid);

        if let Some(new_time) = &section.params.time {
            time = new_time.value;
        }

        if range.start_point.row >= line && line <= range.end_point.row {
            break;
        }
    }

    Some(SectionContext {
        time,
        verses: verses.unwrap_or_default() as u8,
        voices: voices.iter().map(|v| v.value).collect(),
    })
}

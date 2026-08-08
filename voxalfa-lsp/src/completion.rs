use async_lsp::lsp_types::{CompletionItem, CompletionItemKind, Documentation, InsertTextFormat};
use tree_sitter::{Node, Point};
use voxalfa_validator::{
    data_types::{TimeSignature, Voice},
    ts_utils::generated::node_types,
};

use crate::{builtin::*, parameters::*, state::Document};

pub fn get_completion_context(
    doc: &Document,
    line: usize,
    column: usize,
) -> Option<CompletionContext> {
    let root = doc.data.tree.as_ref().map(|t| t.root_node())?;
    let point = Point::new(line, column);
    let target = root.named_descendant_for_point_range(point, point)?;
    let line_str = doc.source.lines().nth(line)?;

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
                let section = get_section_context(doc, line)?;
                let context = CompletionContext::Section(section);

                Some(context)
            }
        }
        node_types::BUILTIN | node_types::PARAMETER_ASSIGNMENT => {
            let builtin = get_builtin_context(target, &doc.source)?;
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
            CompletionContext::Metadata => self.build_param_snippets(HEADER_PARAMS),
            CompletionContext::InitialParams => self.build_param_snippets(INITIAL_PARAMS),
            CompletionContext::SectionParams => self.build_param_snippets(SECTION_PARAMS),
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

    fn build_param_snippets(&self, specs: &[ParamSpec]) -> Vec<CompletionItem> {
        specs
            .iter()
            .map(|spec| CompletionItem {
                label: spec.name.to_string(),
                kind: Some(CompletionItemKind::PROPERTY),
                detail: Some(format!("{}: {}", spec.name, spec.type_str)),
                documentation: Some(Documentation::String(spec.doc.to_string())),
                insert_text_format: Some(InsertTextFormat::SNIPPET),
                insert_text: Some(spec.snippet.to_string()),
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
    pub fn specs(&self) -> &'static [BuiltinValueSpec] {
        match self {
            Builtin::Voice => VOICE_BUILTINS,
            Builtin::Key => KEY_BUILTINS,
            Builtin::Tempo => TEMPO_BUILTINS,
            Builtin::Mark => MARK_BUILTINS,
            Builtin::Touches => TOUCHES_BUILTINS,
            Builtin::Dynamics => DYNAMICS_BUILTINS,
            Builtin::Jump => JUMP_BUILTINS,
        }
    }

    pub fn completion_items(&self) -> Vec<CompletionItem> {
        self.specs()
            .iter()
            .map(|spec| CompletionItem {
                label: spec.label.to_string(),
                kind: Some(CompletionItemKind::ENUM_MEMBER),
                detail: spec.detail.to_string().into(),
                documentation: Some(Documentation::String(spec.doc.to_string())),
                insert_text: Some(spec.label.to_string()),
                ..Default::default()
            })
            .collect()
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
    let identifier = match target.kind_id() {
        node_types::BUILTIN => target.prev_named_sibling()?,
        _ => target.named_child(0)?,
    };
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

    for section in &document.data.body.sections {
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

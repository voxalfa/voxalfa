use async_lsp::lsp_types::{CompletionItem, CompletionItemKind, Documentation, InsertTextFormat};
use tree_sitter::{Node, Point};
use voxalfa_core::{
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
                self.build_document_template_snippet(),
                self.snippet_item("parameters ($)", "Initial Parameters", "[\\$] ${0}"),
                self.snippet_item("metadata (#)", "Header metadata", "[#] ${0}"),
            ],
            CompletionContext::Metadata => self.build_param_snippets(HEADER_PARAMS),
            CompletionContext::InitialParams => self.build_param_snippets(INITIAL_PARAMS),
            CompletionContext::SectionParams => self.build_param_snippets(SECTION_PARAMS),
            CompletionContext::Section(context) => {
                let mut items = Vec::new();

                items.extend(context.build_voice_combo_snippets(1));
                items.extend(context.build_voice_combo_snippets(2));
                items.extend(context.build_voice_combo_snippets(3));

                items.extend(vec![
                    self.snippet_item("1 (verse)", "Verse 1", "[1] ${0}"),
                    self.snippet_item("2 (verse)", "Verse 2", "[2] ${0}"),
                    self.snippet_item("3 (verse)", "Verse 3", "[3] ${0}"),
                    self.snippet_item("parameters ($)", "Section Parameters", "[\\$] ${0}"),
                ]);

                items
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

    fn build_document_template_snippet(&self) -> CompletionItem {
        let snippet = [
            ";; @version 0.1.0-alpha",
            "",
            "[#] title=\"${1:Title}\"",
            "[#] author={\"${2:Author}\"}",
            "[#] description=\"${3:Description}\"",
            "[#] verses={${4:1}}",
            "[#] release={${5:2026}}",
            "[#] language=\"${6:en}\"",
            "[#] tags={\"${7:tag}\"}",
            "",
            "[\\$] key={${8:C}} | time={${9:4,4}} | tempo={${10:100}} | voices={${11:S,A,T,B}}",
            "",
            "---",
            "",
            "[S] |${12:s} :${13:s} !${14:s} :${15:s} ||",
            "[A] |${16:m} :${17:m} !${18:m} :${19:m} ||",
            "[T] |${20:d} :${21:d} !${22:d} :${23:d} ||",
            "[B] |${24:d} :${25:d} !${26:d} :${27:d} ||",
            "",
            "[1] ${28:Lyric}",
            "${0}",
        ]
        .join("\n");

        CompletionItem {
            label: "template".to_string(),
            kind: Some(CompletionItemKind::SNIPPET),
            detail: Some("Voxalfa score template".to_string()),
            insert_text: Some(snippet),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            ..Default::default()
        }
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

const DEFAULT_NOTES: &[&str] = &["s", "m", "d", "d"];

const STANDARD_VOICE_COMBOS: [&[Voice]; 9] = [
    &[Voice::S],
    &[Voice::A],
    &[Voice::T],
    &[Voice::B],
    &[Voice::S, Voice::A],
    &[Voice::T, Voice::B],
    &[Voice::S, Voice::A, Voice::T],
    &[Voice::A, Voice::T, Voice::B],
    &[Voice::S, Voice::A, Voice::T, Voice::B],
];

#[derive(Debug)]
pub struct SectionContext {
    voices: Vec<Voice>,
    used_voices: Vec<Voice>,
    time: TimeSignature,
    verses: u8,
}

impl SectionContext {
    fn build_voice_line(
        &self,
        voice: Voice,
        v_idx: usize,
        measures: usize,
        tab_stop: &mut usize,
    ) -> String {
        let mut line = String::new();
        let note = DEFAULT_NOTES.get(v_idx).copied().unwrap_or("d");

        line.push_str(&format!("[{voice:?}] |"));

        for m in 0..measures {
            for pos in 0..self.time.top as usize {
                line.push_str(&format!("${{{tab_stop}:{note}}}"));
                *tab_stop += 1;

                if pos < (self.time.top as usize - 1) {
                    let next_accent = self.time.get_accent(pos + 1);
                    line.push_str(&format!(" {next_accent}"));
                }
            }

            if m < measures - 1 {
                line.push_str(" | ");
            } else {
                line.push_str(" ||");
            }
        }

        line
    }

    fn build_voice_combo_snippets(&self, measures: usize) -> Vec<CompletionItem> {
        let mut result = STANDARD_VOICE_COMBOS
            .iter()
            .enumerate()
            .filter(|(_, combo)| {
                combo
                    .iter()
                    .all(|v| self.voices.contains(v) && !self.used_voices.contains(v))
            })
            .map(|(combo_idx, combo)| {
                self.build_voice_combo_snippet(combo, measures, combo_idx + 1, None)
            })
            .collect::<Vec<_>>();

        let rest_voices = self
            .voices
            .iter()
            .filter(|v| !self.used_voices.contains(*v))
            .copied()
            .collect::<Vec<_>>();

        if !rest_voices.is_empty() {
            let snippet = self.build_voice_combo_snippet(&rest_voices, measures, 0, Some("lines"));

            result.push(snippet);
        }

        result
    }

    fn build_voice_combo_snippet(
        &self,
        combo: &[Voice],
        measures: usize,
        combo_idx: usize,
        label: Option<&str>,
    ) -> CompletionItem {
        let mut snippet = String::new();
        let mut tab_stop = 1;

        for &voice in combo {
            let orig_idx = self.voices.iter().position(|&v| v == voice).unwrap_or(0);
            let line = self.build_voice_line(voice, orig_idx, measures, &mut tab_stop);
            snippet.push_str(&line);
            snippet.push('\n');
        }

        if self.verses > 0 {
            snippet.push('\n');
            for verse in 0..self.verses {
                snippet.push_str(&format!("[{}] ${{{tab_stop}:~}}\n", verse + 1));
                tab_stop += 1;
            }
        }

        let combo_label = combo.iter().map(|v| format!("{v:?}")).collect::<String>();
        let popup_label = label.unwrap_or(&combo_label);
        let is_single = combo.len() == 1;
        let kind_label = if is_single { "voice" } else { "voices" };

        CompletionItem {
            label: format!("{popup_label} ({kind_label})"),
            kind: Some(CompletionItemKind::SNIPPET),
            detail: Some(format!(
                "Insert line{} for {combo_label} with {} verse(s) ({measures} measure(s) in {}/{})",
                if is_single { "" } else { "s" },
                self.verses,
                self.time.top,
                self.time.bottom
            )),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            insert_text: Some(snippet),
            sort_text: Some(format!("{combo_idx:02}_{measures:02}")),
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
    let mut used_voices = Vec::new();

    for section in &document.data.body.sections {
        let range = document.data.symbols.get_scope_range(section.sid);

        if let Some(new_time) = &section.params.time {
            time = new_time.value;
        }

        if range.start_point.row <= line && line <= range.end_point.row {
            for item in &section.items {
                for solfa in &item.solfa {
                    used_voices.push(solfa.voice);
                }
            }

            break;
        }
    }

    Some(SectionContext {
        time,
        used_voices,
        verses: verses.unwrap_or_default() as u8,
        voices: voices.iter().map(|v| v.value).collect(),
    })
}

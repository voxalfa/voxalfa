use async_lsp::lsp_types::{
    CompletionItem, CompletionItemKind, CompletionList, Documentation, InsertTextFormat,
};
use tree_sitter::{Node, Point};
use voxalfa_core::{
    data_types::{TimeSignature, Voice},
    ts_utils::node_types,
};

use crate::{builtin::*, parameters::*, state::Document};

pub fn get_completion_context(
    doc: &Document,
    line: usize,
    column: usize,
) -> Option<CompletionContext> {
    let root = doc.data.tree.as_ref().map(|t| t.root_node())?;
    let line_str = doc.source.split('\n').nth(line)?; // use split to keep trailing newlines
    let target = resolve_node_at(doc, root, line, column).unwrap_or(root);

    match target.kind_id() {
        node_types::HEADER | node_types::SOURCE_FILE => {
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

fn resolve_node_at<'tree>(
    doc: &Document,
    root: Node<'tree>,
    line: usize,
    column: usize,
) -> Option<Node<'tree>> {
    let point = Point::new(line, column);
    let target = root.named_descendant_for_point_range(point, point)?;

    // source_file is returned at the end position of the document
    // the last node should be used at the end position to give the correct context
    if target.kind_id() != node_types::SOURCE_FILE {
        return Some(target);
    }

    let previous_line = line.checked_sub(1)?;
    let previous_column = doc.source.split('\n').nth(previous_line)?.len();
    let previous_point = Point::new(previous_line, previous_column);
    let previous_target = root.named_descendant_for_point_range(previous_point, previous_point)?;

    if previous_target.kind_id() == node_types::SOURCE_FILE {
        Some(target)
    } else {
        Some(previous_target)
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
    pub fn completion_items(&self) -> CompletionList {
        let items = match self {
            CompletionContext::Header => vec![
                self.build_document_template_snippet(),
                self.snippet_item("parameters ($)", "Initial Parameters", "[\\$] ${0}"),
                self.snippet_item("metadata (#)", "Header metadata", "[#] ${0}"),
            ],
            CompletionContext::Metadata => self.build_param_snippets(HEADER_PARAMS),
            CompletionContext::InitialParams => self.build_param_snippets(INITIAL_PARAMS),
            CompletionContext::SectionParams => self.build_param_snippets(SECTION_PARAMS),
            CompletionContext::Section(context) => self.build_section_snippets(context),
            CompletionContext::Builtin(builtin) => builtin.completion_items(),
        };

        CompletionList {
            is_incomplete: matches!(self, CompletionContext::Section(_)),
            items,
        }
    }

    fn build_section_snippets(&self, context: &SectionContext) -> Vec<CompletionItem> {
        let mut items = Vec::new();

        for measures in 1..=3 {
            items.push(context.build_system_voice_snippet(measures));
        }

        if let Some(voices) = context.parse_voice_line() {
            for measures in 1..=3 {
                items.push(context.build_voice_snippet(&voices, measures));
            }
        }

        items.extend(vec![
            self.snippet_item("verse (1)", "Verse 1", "[1] ${0}"),
            self.snippet_item("verse (2)", "Verse 2", "[2] ${0}"),
            self.snippet_item("verse (3)", "Verse 3", "[3] ${0}"),
            self.snippet_item("parameters ($)", "Section Parameters", "[\\$] ${0}"),
        ]);

        items
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
            ";; @version 0.1.0-alpha.3",
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
    Barline,
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
            Builtin::Barline => BARLINE_BUILTINS,
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

#[derive(Debug)]
pub struct SectionContext {
    line: String,
    voices: Vec<Voice>,
    time: TimeSignature,
    verses: usize,
}

impl SectionContext {
    fn parse_voice_line(&self) -> Option<Vec<Vec<Voice>>> {
        self.line
            .split_whitespace()
            .last()?
            .split('+')
            .map(|split| {
                split
                    .chars()
                    .map(|c| Voice::try_from(c.to_uppercase().to_string()))
                    .collect::<Result<Vec<_>, _>>()
            })
            .collect::<Result<Vec<_>, _>>()
            .ok()
    }

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

    fn build_system_voice_snippet(&self, measures: usize) -> CompletionItem {
        let voices = std::slice::from_ref(&self.voices);
        let group = format_voice_groups(voices);

        let mut item = self.build_voice_snippet(voices, measures);

        item.label = format!("system ({group})");
        item.filter_text = Some("system".to_string());

        item
    }

    fn build_voice_snippet(&self, splits: &[Vec<Voice>], measures: usize) -> CompletionItem {
        let mut snippet = String::new();
        let mut tab_stop = 1;
        let mut voice_idx = 0;

        for (group_id, group) in splits.iter().enumerate() {
            for &voice in group {
                let line = self.build_voice_line(voice, voice_idx, measures, &mut tab_stop);
                voice_idx += 1;

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

            if group_id != splits.len() - 1 {
                snippet.push_str("\n++\n\n");
            }
        }

        let expression = self
            .line
            .split_whitespace()
            .last()
            .unwrap_or_default()
            .to_string();

        let groups = format_voice_groups(splits);
        let is_single = splits.len() == 1;

        CompletionItem {
            label: format!("{expression} ({groups})"),
            filter_text: Some(expression.clone()),
            kind: Some(CompletionItemKind::SNIPPET),
            detail: Some(format!(
                "Insert line{} for {} with {} verse(s) ({measures} measure(s) in {}/{})",
                if is_single { "" } else { "s" },
                expression.to_uppercase(),
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

fn format_voice_groups(groups: &[Vec<Voice>]) -> String {
    groups
        .iter()
        .map(|group| {
            group
                .iter()
                .map(|voice| format!("{voice:?}"))
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("/")
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
        "barline" => Some(Builtin::Barline),
        _ => None,
    }
}

fn get_section_context(document: &Document, line: usize) -> Option<SectionContext> {
    let header = &document.data.header;
    let verses = header.metadata.verses.clone().map(|v| v.value);

    let mut time = header.params.time.clone()?.value;

    for section in &document.data.body.sections {
        if let Some(new_time) = &section.params.time {
            time = new_time.value;
        }
    }

    let voices = header
        .params
        .voices
        .as_ref()?
        .value
        .iter()
        .map(|voice| voice.value)
        .collect();

    Some(SectionContext {
        time,
        line: document.rope.line(line).to_string(),
        voices,
        verses: verses.unwrap_or_default(),
    })
}

#[cfg(test)]
mod tests {
    use async_lsp::lsp_types::Url;
    use voxalfa_core::validation::Validator;

    use super::*;

    fn document(source: &str) -> Document {
        let mut validator = Validator::init().unwrap();
        let data = validator.analyze(source);

        Document::new(
            Url::parse("file:///test.vfa").unwrap(),
            source.to_string(),
            data,
        )
    }

    #[test]
    fn test_initial_completion_context() {
        let doc = document("");
        let context = get_completion_context(&doc, 0, 0);

        assert!(matches!(context, Some(CompletionContext::Header)));
    }

    #[test]
    fn test_header_completion_context() {
        let doc = document("[#] title=\"test\"\n");
        let context = get_completion_context(&doc, 1, 0);

        assert!(matches!(context, Some(CompletionContext::Header)));
    }

    #[test]
    fn test_params_completion_context() {
        let doc = document("[$]  ");
        let context = get_completion_context(&doc, 0, 4);

        assert!(
            matches!(context, Some(CompletionContext::InitialParams)),
            "{context:?}"
        );
    }

    #[test]
    fn test_builtin_completion_context() {
        let doc = document("[$] dynamics={}");
        let context = get_completion_context(&doc, 0, 14);

        assert!(matches!(
            context,
            Some(CompletionContext::Builtin(Builtin::Dynamics))
        ),);
    }

    #[test]
    fn test_section_completion_context() {
        let doc = document("[$] voices={S} time={3,4}\n\n---\n");
        let context = get_completion_context(&doc, 3, 0);

        assert!(matches!(
            context,
            Some(CompletionContext::Section(SectionContext { .. }))
        ),);
    }

    #[test]
    fn test_end_position_completion_context() {
        let doc = document("[#] title=\"test\"\n\n");
        let context = get_completion_context(&doc, 2, 0);

        assert!(matches!(context, Some(CompletionContext::Header)));
    }

    #[test]
    fn test_voice_snippet_input() {
        let cases = [
            "S", "SA", "SAT", "SATB", "satb", "SA+TB", "sa+tb", "S+ATB", "S+A+T+B",
        ];

        for input in cases {
            let source = format!("[$] voices={{S,A,T,B}} time={{4,4}}\n\n---\n\n{input}");
            let doc = document(&source);

            let result = get_completion_context(&doc, 4, input.len())
                .unwrap()
                .completion_items();

            let snippet_match = result
                .items
                .iter()
                .any(|item| item.label.to_lowercase().starts_with(&input.to_lowercase()));

            assert!(result.is_incomplete);
            assert!(snippet_match,);
        }
    }
}

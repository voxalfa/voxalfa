use tree_sitter::{Node, QueryCursor, StreamingIterator};

use crate::{
    ast::{
        solfa::{Measure, MeasureState, MeasureToken, SolfaLine},
        symbols::{
            Assignment, AssignmentData, Document, Field, FieldAssign, Header, KeyData, Range,
            Section, ValueData, ValueKind,
        },
        types::Voice,
    },
    diagnostic::{Diagnostic, DiagnosticKind, DiagnosticLevel},
    ts_utils::{context::TSContext, parsing::ParseNode, types::AssignmentDataSource},
};

#[derive(Debug)]
pub struct DocumentValidator<'a> {
    pub diagnostics: Vec<Diagnostic>,
    pub source: &'a [u8],
    pub output: Document,
}

impl<'a> DocumentValidator<'a> {
    pub fn new(source: &'a str) -> Self {
        Self {
            source: source.as_bytes(),
            diagnostics: Vec::default(),
            output: Document::default(),
        }
    }

    pub fn validate(mut self, context: &mut TSContext) -> (Document, Vec<Diagnostic>) {
        if let Some(tree) = context.parse(self.source) {
            let root = tree.root_node();

            self.handle_parser_errors(root, context);
            self.handle_root_node(root);
        }

        (self.output, self.diagnostics)
    }

    fn handle_root_node(&mut self, root: Node<'_>) {
        for child in root.named_children(&mut root.walk()) {
            match child.kind() {
                "inline_comment" => {} // TODO: Handle language directives
                "header" => self.handle_header_node(child),
                "body" => self.handle_body_node(child),
                _ => {}
            }
        }
    }

    fn handle_header_node(&mut self, node: Node<'_>) {
        let mut header = Header::default();

        for child in node.named_children(&mut node.walk()) {
            match child.kind() {
                "metadata_line" => self.handle_assignment_node(child, &mut header.metadata),
                "parameter_line" => self.handle_assignment_node(child, &mut header.params),
                _ => {}
            }
        }

        self.output.header = header;
    }

    fn handle_assignment_node<T: FieldAssign>(&mut self, node: Node<'_>, params: &mut T) {
        for child in node.named_children(&mut node.walk()) {
            if let Some(source) = self.resolve_assignment_source(child) {
                params.assign_field(source, self);
            }
        }
    }

    fn resolve_assignment_source<'b>(
        &mut self,
        node: Node<'b>,
    ) -> Option<AssignmentDataSource<'b>> {
        let key_node = node.child_by_field_name("name")?;
        let value_node = node.child_by_field_name("value")?;

        let key_data = KeyData {
            name: self.resolve_node_string(key_node)?,
            range: key_node.range(),
        };

        let value_data = self.resolve_value_data(value_node)?;

        let data = AssignmentData {
            range: node.range(),
            key: key_data,
            value: value_data,
        };

        Some(AssignmentDataSource {
            key_node,
            value_node,
            data,
        })
    }

    fn handle_body_node(&mut self, node: Node<'_>) {
        for child in node.named_children(&mut node.walk()) {
            if child.kind() == "section" {
                let section = self.resolve_section(child);
                self.output.body.sections.push(section);
            }
        }
    }

    fn handle_solfa_node(&mut self, node: Node<'_>, lines: &mut Vec<SolfaLine>) {
        if let Some(value) = self.resolve_solfa_line(node, lines.len()) {
            lines.push(value);
        }
    }

    fn resolve_solfa_line(&mut self, node: Node<'_>, id: usize) -> Option<SolfaLine> {
        let voice = self.resolve_solfa_voice(node, id)?;
        let mut measures = Vec::new();

        for measure in node.children_by_field_name("measure", &mut node.walk()) {
            if let Some(measure) = self.resolve_measure(measure) {
                measures.push(measure);
            }
        }

        Some(SolfaLine {
            voice,
            measures,
            range: node.range(),
        })
    }

    fn resolve_solfa_voice(&mut self, node: Node<'_>, id: usize) -> Option<Voice> {
        let voice_node = node.child_by_field_name("voice")?;
        let voice_str = self.resolve_node_string(voice_node)?;
        let voice = Voice::try_from(voice_str.as_str());

        if let Ok(value) = voice {
            if let Some(expected) = self.output.get_voice(id) {
                if value != expected {
                    self.report_error(
                        voice_node.range(),
                        DiagnosticKind::VoiceMismatch(expected, value),
                    );
                }

                return Some(expected);
            }

            self.report_error(
                voice_node.range(),
                DiagnosticKind::UndefinedVoice(voice_str),
            );
        } else {
            self.report_error(voice_node.range(), DiagnosticKind::InvalidVoice(voice_str));
        }

        None
    }

    fn resolve_measure(&mut self, node: Node<'_>) -> Option<Measure> {
        let mut measure = Measure::new(node.range());
        let mut state = MeasureState::default();

        for child in node.named_children(&mut node.walk()) {
            let range = child.range();
            let token = self.resolve_measure_token(child);

            if state.col_start.is_none() {
                state.col_start = Some(range);
            }

            match token {
                Some(MeasureToken::NormalDivision | MeasureToken::MediumDivision) => {
                    self.validate_measure_column(&state);
                    state.col_acc = vec![0];
                    state.col_start = None;
                    state.col_count += 1;
                }
                Some(MeasureToken::HalfDivision) => {
                    state.col_acc.push(0);
                }
                Some(
                    MeasureToken::Note(_) | MeasureToken::ProlongedNote | MeasureToken::EmptyNote,
                ) => {
                    if let Some(last) = state.col_acc.last_mut() {
                        *last += 1;
                    }
                } // note
                _ => {}
            }

            state.col_end = Some(range);

            if let Some(value) = token {
                measure.tokens.push(value);
            }
        }

        state.col_count += 1;

        if let Some(time) = &self.output.header.params.time
            && state.col_count != time.value.top
        {
            self.report_error(
                node.range(),
                DiagnosticKind::MeasureColumnMismatch(
                    time.value.top,
                    state.col_count,
                    time.data.range,
                ),
            );
        }

        self.validate_measure_column(&state);

        Some(measure)
    }

    fn validate_measure_column(&mut self, state: &MeasureState) {
        if state.col_acc.len() > 1 || state.col_acc[0] == 1 {
            return;
        }

        if let Some((start_range, end_range)) = state.col_start.zip(state.col_end) {
            self.report_error(
                Range {
                    start_byte: start_range.start_byte,
                    end_byte: end_range.end_byte,
                    start_point: start_range.start_point,
                    end_point: end_range.end_point,
                },
                DiagnosticKind::InvalidNoteDistribution,
            );
        }
    }

    fn resolve_measure_token(&mut self, node: Node<'_>) -> Option<MeasureToken> {
        match node.kind() {
            "half_division" => Some(MeasureToken::HalfDivision),
            "quarter_division" => Some(MeasureToken::QuarterDivision),
            "medium_division" => Some(MeasureToken::MediumDivision),
            "normal_division" => Some(MeasureToken::NormalDivision),
            "underline_start" => Some(MeasureToken::UnderlineStart),
            "underline_end" => Some(MeasureToken::UnderlineEnd),
            "pulse" => self.resolve_pulse_node(node),
            _ => None,
        }
    }

    fn resolve_pulse_node(&mut self, node: Node<'_>) -> Option<MeasureToken> {
        let child = node.named_child(0)?;

        match child.kind() {
            "note" => self.parse_node(child).map(MeasureToken::Note),
            "empty_note" => Some(MeasureToken::EmptyNote),
            "prolonged_note" => Some(MeasureToken::ProlongedNote),
            _ => None,
        }
    }

    fn resolve_section(&mut self, node: Node<'_>) -> Section {
        let mut section = Section::default();

        for child in node.named_children(&mut node.walk()) {
            match child.kind() {
                "parameter_line" => self.handle_assignment_node(child, &mut section.params),
                "dynamics_line" => self.handle_assignment_node(child, &mut section.dynamics),
                "solfa_line" => self.handle_solfa_node(child, &mut section.solfa),
                "lyric_line" => {}
                _ => {}
            }
        }

        if let Some(voices) = &self.output.header.params.voices {
            let count = section.solfa.len();
            let expected = voices.value.len();

            if count != expected {
                let start_range = section.solfa.first().map(|line| line.range);
                let end_range = section.solfa.last().map(|line| line.range);

                let range = start_range
                    .zip(end_range)
                    .map(|(start, end)| Range {
                        start_byte: start.start_byte,
                        end_byte: end.end_byte,
                        start_point: start.start_point,
                        end_point: end.end_point,
                    })
                    .unwrap_or(node.range());

                self.report_error(
                    range,
                    DiagnosticKind::VoiceCountMismatch(expected, count, voices.data.range),
                );
            }
        }

        section
    }

    fn handle_parser_errors(&mut self, root: Node<'_>, context: &mut TSContext) {
        let capture_names = context.error_query.capture_names();

        let mut cursor = QueryCursor::new();
        let mut matches = cursor.matches(&context.error_query, root, self.source);

        while let Some(m) = matches.next() {
            for capture in m.captures.iter() {
                let node = capture.node;
                let name = capture_names[capture.index as usize];

                self.handle_capture(name, node);
            }
        }
    }

    fn handle_capture(&mut self, name: &str, node: Node<'_>) {
        match name {
            "error.syntax" => {
                self.report_error(node.range(), DiagnosticKind::SyntaxError);
            }
            "error.missing" => {
                self.report_error(node.range(), DiagnosticKind::Missing(node.kind().into()));
            }
            _ => {}
        }
    }

    fn resolve_value_data(&mut self, node: Node<'_>) -> Option<ValueData> {
        let kind = match node.kind() {
            "string" => ValueKind::String,
            "integer" => ValueKind::Integer,
            "float" => ValueKind::Float,
            "boolean" => ValueKind::Boolean,
            "list" => ValueKind::List,
            "token" => ValueKind::Token,
            _ => return None,
        };

        Some(ValueData {
            kind,
            range: node.range(),
        })
    }

    #[inline]
    pub(crate) fn parse_node<T: ParseNode>(&mut self, node: Node<'_>) -> Option<T> {
        T::parse_node(node, self)
    }

    pub(crate) fn assign_field<T: ParseNode>(
        &mut self,
        source: AssignmentDataSource,
        field: &mut Field<T>,
    ) {
        if let Some(value) = field {
            self.report_warning(
                source.data.range,
                DiagnosticKind::KeyReassignment(source.data.key.name.clone(), value.data.range),
            );
        } else {
            *field = self.parse_node(source.value_node).map(|value| Assignment {
                value,
                data: source.data,
            });
        }
    }

    pub(crate) fn resolve_node_string(&mut self, node: Node<'_>) -> Option<String> {
        node.utf8_text(self.source)
            .map(String::from)
            .inspect_err(|e| self.report_error(node.range(), DiagnosticKind::InvalidUTF8(*e)))
            .ok()
    }

    pub(crate) fn report_error(&mut self, range: Range, kind: DiagnosticKind) {
        self.diagnostics.push(Diagnostic {
            level: DiagnosticLevel::Error,
            kind,
            range,
        });
    }

    pub(crate) fn report_warning(&mut self, range: Range, kind: DiagnosticKind) {
        self.diagnostics.push(Diagnostic {
            level: DiagnosticLevel::Warning,
            kind,
            range,
        });
    }
}

use tree_sitter::{Node, QueryCursor, StreamingIterator};

use crate::{
    ast::{
        body::{Body, Section},
        document::Document,
        header::Header,
        lyrics::{
            LyricAnchor, LyricChunk, LyricChunkKind, LyricColumn, LyricLine, LyricOperator,
            LyricOperatorKind, LyricToken,
        },
        solfa::{Measure, MeasureState, MeasureToken, MeasureTokenKind, SolfaLine},
        symbols::{Field, FieldAssign, ScopeId, ScopeKind, SymbolKind, SymbolRef, SymbolTree},
        types::Voice,
    },
    diagnostic::{Diagnostic, DiagnosticKind, DiagnosticLevel},
    ts_utils::{
        context::TSContext,
        parsing::ParseNode,
        range::{Range, RangeMerge},
        types::AssignmentData,
    },
};

#[derive(Debug)]
pub struct ValidatorOutput {
    pub tree: SymbolTree,
    pub document: Document,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Debug)]
pub struct DocumentValidator<'a> {
    pub source: &'a [u8],
    pub tree: SymbolTree,
    pub document: Document,
    pub diagnostics: Vec<Diagnostic>,
}

impl<'a> DocumentValidator<'a> {
    pub fn new(source: &'a str) -> Self {
        Self {
            source: source.as_bytes(),
            diagnostics: Vec::default(),
            document: Document::default(),
            tree: SymbolTree::default(),
        }
    }

    pub fn validate(mut self, context: &mut TSContext) -> ValidatorOutput {
        if let Some(tree) = context.parse(self.source) {
            let root = tree.root_node();

            self.handle_parser_errors(root, context);
            self.handle_root_node(root);
        }

        ValidatorOutput {
            tree: self.tree,
            document: self.document,
            diagnostics: self.diagnostics,
        }
    }

    fn handle_root_node(&mut self, root: Node<'_>) {
        for child in root.named_children(&mut root.walk()) {
            match child.kind() {
                "header" => self.handle_header_node(child),
                "body" => self.handle_body_node(child),
                _ => {}
            }
        }
    }

    fn handle_header_node(&mut self, node: Node<'_>) {
        let sid = self.tree.add_scope(ScopeKind::Header, node.range(), None);
        let mut header = Header::new(sid);

        for child in node.named_children(&mut node.walk()) {
            match child.kind() {
                "metadata_line" => self.handle_assignment_node(child, sid, &mut header.metadata),
                "parameter_line" => self.handle_assignment_node(child, sid, &mut header.params),
                _ => {}
            }
        }

        self.document.header = header;
    }

    fn handle_assignment_node<T: FieldAssign>(
        &mut self,
        node: Node<'_>,
        parent_sid: ScopeId,
        params: &mut T,
    ) {
        let root_sid =
            self.tree
                .add_scope(ScopeKind::AssignmentLine, node.range(), parent_sid.into());

        for child in node.named_children(&mut node.walk()) {
            let scope_id =
                self.tree
                    .add_scope(ScopeKind::Assignment, child.range(), root_sid.into());

            if let Some(source) = self.resolve_assignment_data(child, scope_id) {
                params.assign_field(source, self);
            }
        }
    }

    fn handle_body_node(&mut self, node: Node<'_>) {
        let sid = self.tree.add_scope(ScopeKind::Header, node.range(), None);
        let mut body = Body::new(sid);

        for child in node.named_children(&mut node.walk()) {
            if child.kind() == "section" {
                let section = self.resolve_section(child, body.sid);
                body.sections.push(section);
            }
        }

        self.document.body = body;
    }

    fn handle_solfa_node(
        &mut self,
        node: Node<'_>,
        parent_sid: ScopeId,
        lines: &mut Vec<SolfaLine>,
    ) {
        let sid = self
            .tree
            .add_scope(ScopeKind::SolfaLine, node.range(), parent_sid.into());

        if let Some(value) = self.resolve_solfa_line(node, sid, lines.len()) {
            self.validate_solfa_line(&value);
            lines.push(value);
        }
    }

    fn resolve_solfa_line(
        &mut self,
        node: Node<'_>,
        scope_id: ScopeId,
        id: usize,
    ) -> Option<SolfaLine> {
        let voice = self.resolve_solfa_voice(node, id)?;
        let mut measures = Vec::new();

        for measure in node.children_by_field_name("measure", &mut node.walk()) {
            let scope_id =
                self.tree
                    .add_scope(ScopeKind::Measure, measure.range(), scope_id.into());

            if let Some(measure) = self.resolve_measure(measure, scope_id) {
                measures.push(measure);
            }
        }

        let line = SolfaLine {
            sid: scope_id,
            voice,
            measures,
        };

        Some(line)
    }

    fn handle_lyric_node(&mut self, node: Node<'_>, parent_sid: ScopeId, section: &mut Section) {
        let sid = self
            .tree
            .add_scope(ScopeKind::LyricLine, node.range(), parent_sid.into());

        if let Some(line) = self.resolve_lyric_line(node, sid, section) {
            self.validate_lyric_line(&line);
            section.lyrics.push(line);
        }
    }

    fn resolve_lyric_line(
        &mut self,
        node: Node<'_>,
        scope_id: ScopeId,
        section: &Section,
    ) -> Option<LyricLine> {
        let verse_node = node.child_by_field_name("verse")?;
        let content_node = node.child_by_field_name("content")?;
        let anchor_node = node.child_by_field_name("anchor");

        let group = section.solfa.len();
        let verse = self.parse_node(verse_node)?;
        let expected_verse = section.lyrics.iter().filter(|l| l.group == group).count() + 1;
        let tokens = self.resolve_lyric_tokens(content_node, scope_id);
        let anchor = anchor_node.and_then(|n| self.resolve_lyric_anchor(n, scope_id));

        if verse != expected_verse {
            self.report_warning(
                verse_node.range(),
                DiagnosticKind::MismatchedVerseIndex(expected_verse, verse),
            );
        }

        Some(LyricLine {
            sid: scope_id,
            verse: expected_verse,
            group,
            anchor,
            tokens,
        })
    }

    fn resolve_lyric_tokens(&mut self, node: Node<'_>, scope_id: ScopeId) -> Vec<LyricToken> {
        let mut tokens = Vec::new();

        for child in node.named_children(&mut node.walk()) {
            match child.kind() {
                "lyric_column" => {
                    let column = self.resolve_lyric_column(child, scope_id);
                    tokens.push(LyricToken::Column(column));
                }
                _ => {
                    if let Some(operator) = self.resolve_lyric_operator(child, scope_id) {
                        tokens.push(LyricToken::Operator(operator));
                    }
                }
            }
        }

        tokens
    }

    fn resolve_lyric_operator(
        &mut self,
        node: Node<'_>,
        scope_id: ScopeId,
    ) -> Option<LyricOperator> {
        let value = match node.kind() {
            "space_operator" => LyricOperatorKind::Space,
            "concat_operator" => LyricOperatorKind::Concat,
            "newline_operator" => LyricOperatorKind::Newline,
            _ => return None,
        };

        let sid = self
            .tree
            .add_symbol(SymbolKind::Token, node.range(), scope_id);

        Some(LyricOperator { sid, value })
    }

    fn resolve_lyric_column(&mut self, node: Node<'_>, scope_id: ScopeId) -> LyricColumn {
        let mut chunks = Vec::new();

        if let Some(lyric_node) = node.child_by_field_name("lyric") {
            for child in lyric_node.named_children(&mut node.walk()) {
                if let Some(value) = self.resolve_lyric_atom(child, scope_id) {
                    chunks.push(value);
                }
            }
        }

        let extra_span = node
            .child_by_field_name("span")
            .and_then(|s| self.resolve_node_string(s))
            .map(|s| s.len())
            .unwrap_or_default();

        LyricColumn {
            span: extra_span + 1,
            chunks,
        }
    }

    fn resolve_lyric_atom(&mut self, node: Node<'_>, scope_id: ScopeId) -> Option<LyricChunk> {
        let value = match node.kind() {
            "space_operator" => LyricChunkKind::Space,
            "concat_operator" => LyricChunkKind::Concat,
            "newline_operator" => LyricChunkKind::Newline,
            "underline_marker" => LyricChunkKind::UnderlineMarker,
            "lyric_placeholder" => LyricChunkKind::Placeholder,
            "lyric_string" => LyricChunkKind::String(self.resolve_node_string(node)?),
            _ => return None,
        };

        let sid = self
            .tree
            .add_symbol(SymbolKind::Token, node.range(), scope_id);

        Some(LyricChunk { sid, value })
    }

    fn resolve_lyric_anchor(&mut self, node: Node<'_>, scope_id: ScopeId) -> Field<LyricAnchor> {
        let value = match node.kind() {
            "space_anchor" => LyricAnchor::Space,
            "concat_anchor" => LyricAnchor::Concat,
            "newline_anchor" => LyricAnchor::Newline,
            _ => return None,
        };

        let sid = self
            .tree
            .add_symbol(SymbolKind::Token, node.range(), scope_id);

        Some(SymbolRef { sid, value })
    }

    // FIXME: Check column count against measures
    fn validate_lyric_line(&mut self, line: &LyricLine) {
        let mut current_underline = None;

        let columns = line.tokens.iter().filter_map(|t| match t {
            LyricToken::Column(value) => Some(value),
            LyricToken::Operator(_) => None,
        });

        for column in columns.flat_map(|c| &c.chunks) {
            if matches!(column.value, LyricChunkKind::UnderlineMarker)
                && current_underline.take().is_none()
            {
                current_underline = Some(column);
            }
        }

        if let Some(token) = current_underline {
            let range = self
                .tree
                .get_symbol_range(token.sid)
                .merge(self.tree.get_scope_range(line.sid));

            self.report_error(range, DiagnosticKind::UnmatchedUnderline);
        }
    }

    fn resolve_solfa_voice(&mut self, node: Node<'_>, id: usize) -> Option<Voice> {
        let voice_node = node.child_by_field_name("voice")?;
        let voice_str = self.resolve_node_string(voice_node)?;
        let voice = Voice::try_from(voice_str.as_str());

        if let Ok(value) = voice {
            if let Some(expected) = self.document.get_voice(id) {
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

    fn resolve_measure(&mut self, node: Node<'_>, scope_id: ScopeId) -> Option<Measure> {
        let mut measure = Measure::new(scope_id);
        let mut state = MeasureState::new();

        for child in node.named_children(&mut node.walk()) {
            let prev_token = measure.tokens.last();
            let token = self.resolve_measure_token(child, scope_id, prev_token, &mut state);

            if let Some(token) = token {
                if token.value.is_beat_separator() {
                    self.validate_measure_column(&state);
                }

                measure.tokens.push(token);
            }
        }

        state.finalize();

        self.validate_measure_column(&state);
        self.validate_time_signature(&state, node);

        Some(measure)
    }

    fn resolve_measure_token(
        &mut self,
        node: Node<'_>,
        scope_id: ScopeId,
        prev_token: Option<&MeasureToken>,
        state: &mut MeasureState,
    ) -> Option<MeasureToken> {
        let range = node.range();
        let kind = self.resolve_measure_token_kind(node)?;
        let sid = self.tree.add_symbol(SymbolKind::Token, range, scope_id);

        state.update_range(range);

        // insert virtual empty notes
        if kind.is_beat_boundary()
            && (prev_token.is_some_and(|t| t.value.is_beat_boundary()) || state.is_empty())
        {
            state.append_note();
        }

        if matches!(kind, MeasureTokenKind::HalfDivision) {
            state.divide();
        } else if kind.is_beat_separator() {
            state.next_column();
        } else if kind.is_note() {
            state.append_note();
        }

        Some(MeasureToken { sid, value: kind })
    }

    fn validate_time_signature(&mut self, state: &MeasureState, node: Node<'_>) {
        if let Some(time) = &self.document.header.params.time
            && state.col_count != time.value.top
        {
            let scope = self.tree.resolve_scope(time.sid);

            self.report_error(
                node.range(),
                DiagnosticKind::MeasureColumnMismatch(time.value.top, state.col_count, scope.range),
            );
        }
    }

    fn validate_measure_column(&mut self, state: &MeasureState) {
        if state.is_valid() {
            return;
        }

        if let Some((start_range, end_range)) = state.col_start.zip(state.col_end) {
            self.report_error(
                start_range.merge(end_range),
                DiagnosticKind::InvalidNoteDistribution,
            );
        }
    }

    fn validate_solfa_line(&mut self, line: &SolfaLine) {
        let mut current_underline = None;

        for measure in &line.measures {
            for token in &measure.tokens {
                if matches!(token.value, MeasureTokenKind::UnderlineMarker)
                    && current_underline.take().is_none()
                {
                    current_underline = Some(token);
                }
            }
        }

        if let Some(token) = current_underline {
            let scope_range = self.tree.get_scope_range(line.sid);
            let token_range = self.tree.get_symbol_range(token.sid);

            self.report_error(
                token_range.merge(scope_range),
                DiagnosticKind::UnmatchedUnderline,
            );
        }
    }

    fn resolve_measure_token_kind(&mut self, node: Node<'_>) -> Option<MeasureTokenKind> {
        match node.kind() {
            "half_division" => Some(MeasureTokenKind::HalfDivision),
            "quarter_division" => Some(MeasureTokenKind::QuarterDivision),
            "medium_division" => Some(MeasureTokenKind::MediumDivision),
            "normal_division" => Some(MeasureTokenKind::NormalDivision),
            "underline_marker" => Some(MeasureTokenKind::UnderlineMarker),
            "pulse" => self.resolve_pulse_node(node),
            _ => None,
        }
    }

    fn resolve_pulse_node(&mut self, node: Node<'_>) -> Option<MeasureTokenKind> {
        let child = node.named_child(0)?;

        match child.kind() {
            "note" => self.parse_node(child).map(MeasureTokenKind::Note),
            "empty_note" => Some(MeasureTokenKind::EmptyNote),
            "prolonged_note" => Some(MeasureTokenKind::ProlongedNote),
            _ => None,
        }
    }

    fn resolve_section(&mut self, node: Node<'_>, parent_sid: ScopeId) -> Section {
        let sid = self
            .tree
            .add_scope(ScopeKind::Section, node.range(), parent_sid.into());

        let mut section = Section::new(sid);

        for child in node.named_children(&mut node.walk()) {
            match child.kind() {
                "parameter_line" => self.handle_assignment_node(child, sid, &mut section.params),
                "dynamics_line" => self.handle_assignment_node(child, sid, &mut section.dynamics),
                "solfa_line" => self.handle_solfa_node(child, sid, &mut section.solfa),
                "lyric_line" => self.handle_lyric_node(child, sid, &mut section),
                _ => {}
            }
        }

        self.validate_voice_count(&section, node.range());
        self.validate_masure_count(&section);

        section
    }

    fn validate_voice_count(&mut self, section: &Section, range: Range) {
        let Some(voices) = &self.document.header.params.voices else {
            return;
        };

        let count = section.solfa.len();
        let expected = voices.value.len();

        if count != expected {
            self.report_error(
                range,
                DiagnosticKind::VoiceCountMismatch(
                    expected,
                    count,
                    self.tree.get_symbol_range(voices.sid),
                ),
            );
        }
    }

    fn validate_masure_count(&mut self, section: &Section) {
        if let Some(first) = section.solfa.first() {
            for line in section.solfa.iter().skip(1) {
                let expected = first.measures.len();
                let count = line.measures.len();
                let first_range = self.tree.get_scope_range(first.sid);
                let current_range = self.tree.get_scope_range(line.sid);

                if expected != count {
                    self.report_error(
                        current_range,
                        DiagnosticKind::MeasureCountMismatch(expected, count, first_range),
                    );
                }
            }
        }
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

    fn resolve_assignment_data<'b>(
        &mut self,
        node: Node<'b>,
        scope_id: ScopeId,
    ) -> Option<AssignmentData<'b>> {
        let key_node = node.child_by_field_name("name")?;
        let value_node = node.child_by_field_name("value")?;
        let key_name = self.resolve_node_string(key_node)?;

        Some(AssignmentData {
            scope_id,
            value_node,
            key_node,
            key_name,
            full_range: node.range(),
            key_range: key_node.range(),
            value_range: value_node.range(),
        })
    }

    #[inline]
    pub(crate) fn parse_node<T: ParseNode>(&mut self, node: Node<'_>) -> Option<T> {
        T::parse_node(node, self)
    }

    pub(crate) fn assign_field<T: ParseNode>(
        &mut self,
        data: AssignmentData,
        field: &mut Field<T>,
    ) {
        if let Some(value) = field {
            let name = data.key_name.clone();
            let scope = self.tree.resolve_scope(value.sid);

            self.report_warning(
                data.full_range,
                DiagnosticKind::KeyReassignment(name, scope.range),
            );
        }

        let _ = self.tree.add_symbol(
            SymbolKind::Key(data.key_name.clone()),
            data.key_range,
            data.scope_id,
        );

        let sid = self
            .tree
            .add_symbol(T::symbol_kind(), data.value_range, data.scope_id);

        *field = self
            .parse_node(data.value_node)
            .map(|value| SymbolRef { sid, value });
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

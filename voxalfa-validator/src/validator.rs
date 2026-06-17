use tree_sitter::{Node, QueryCursor, StreamingIterator};

use crate::{
    ast::{
        body::{Body, Section},
        document::Document,
        header::Header,
        lyrics::{
            LyricAnchor, LyricChunk, LyricChunkKind, LyricColumn, LyricLine, LyricOperator,
            LyricOperatorKind, LyricSpecialChar, LyricToken,
        },
        solfa::{Note, Pulse, PulseAccent, PulseToken, PulseTokenKind, SolfaLine},
        symbols::{Field, FieldAssign, ScopeId, ScopeKind, SymbolKind, SymbolRef, SymbolTree},
        types::Voice,
    },
    diagnostic::{Diagnostic, DiagnosticKind, DiagnosticLevel},
    ir::{
        DocumentIR, SectionIR,
        solfa::{PulseColumnKind, PulseIR, SolfaLineIR, UnderlineRange},
    },
    ts_utils::{
        context::TSContext,
        generated::node_types,
        parsing::ParseNode,
        range::{Range, RangeMerge},
        types::AssignmentData,
    },
};

#[derive(Debug)]
pub struct ValidatorOutput {
    pub tree: SymbolTree,
    pub document: Document,
    pub ir: DocumentIR,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Debug)]
pub struct DocumentValidator<'a> {
    pub source: &'a [u8],
    pub tree: SymbolTree,
    pub document: Document,
    pub ir: DocumentIR,
    pub diagnostics: Vec<Diagnostic>,
}

impl<'a> DocumentValidator<'a> {
    pub fn new(source: &'a str) -> Self {
        Self {
            source: source.as_bytes(),
            document: Document::default(),
            tree: SymbolTree::default(),
            ir: DocumentIR::default(),
            diagnostics: Vec::default(),
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
            ir: self.ir,
            diagnostics: self.diagnostics,
        }
    }

    fn handle_root_node(&mut self, root: Node<'_>) {
        for child in root.named_children(&mut root.walk()) {
            match child.kind_id() {
                node_types::HEADER => self.handle_header_node(child),
                node_types::BODY => self.handle_body_node(child),
                _ => {}
            }
        }
    }

    fn handle_header_node(&mut self, node: Node<'_>) {
        let sid = self.tree.add_scope(ScopeKind::Header, node.range(), None);
        let mut header = Header::new(sid);

        for child in node.named_children(&mut node.walk()) {
            match child.kind_id() {
                node_types::METADATA_LINE => {
                    self.handle_assignment_node(child, sid, &mut header.metadata)
                }
                node_types::PARAMETER_LINE => {
                    self.handle_assignment_node(child, sid, &mut header.params)
                }
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
            if child.kind_id() == node_types::SECTION {
                let section = self.resolve_section(child, body.sid);
                let section_ir = self.resolve_section_ir(&section);
                body.sections.push(section);
                self.ir.sections.push(section_ir);
            }
        }

        self.document.body = body;
    }

    fn resolve_section_ir(&mut self, section: &Section) -> SectionIR {
        let mut section_ir = SectionIR::default();

        for line in &section.solfa {
            let line_ir = self.resolve_line_ir(line);
            section_ir.lines.push(line_ir);
        }

        section_ir
    }

    fn resolve_line_ir(&mut self, line: &SolfaLine) -> SolfaLineIR {
        let mut line_ir = SolfaLineIR::default();
        let mut underline_pos = None;
        let mut underline_sid = None;
        let mut column_offset = 0;

        for pulse in &line.pulses {
            let mut pulse_ir = PulseIR::new(pulse.accent);
            let mut prev_notes = Vec::new();
            let mut stream = pulse.tokens.iter().peekable();
            let mut current_acc = 1.;
            let mut last_acc = 0.;

            while let Some(token) = stream.next() {
                match &token.value {
                    PulseTokenKind::Note(note) => {
                        prev_notes.push(*note);

                        if let Some(next_token) = stream.peek() {
                            match next_token.value {
                                PulseTokenKind::Note(_) => continue,
                                PulseTokenKind::ProlongedNote => {
                                    let range = self.tree.get_symbol_range(next_token.sid);
                                    self.report_error(range, DiagnosticKind::SyntaxError);
                                }
                                _ => {}
                            }
                        }

                        let notes = std::mem::take(&mut prev_notes);
                        pulse_ir.add_column(PulseColumnKind::Notes(notes));
                    }
                    PulseTokenKind::ProlongedNote => {
                        if let Some(last_note) = self.resolve_last_note(&line_ir) {
                            pulse_ir.add_column(PulseColumnKind::ProlongedNote(last_note));
                        } else {
                            let range = self.tree.get_symbol_range(token.sid);
                            self.report_error(range, DiagnosticKind::InvalidNoteProlongation);
                        }
                    }
                    PulseTokenKind::HalfDivision => {
                        last_acc = current_acc;
                        current_acc = 0.5;
                    }
                    PulseTokenKind::QuarterDivision => {
                        last_acc = current_acc;
                        current_acc = 0.25;
                    }
                    PulseTokenKind::UnderlineMarker => {
                        underline_sid = Some(token.sid);

                        match underline_pos.take() {
                            Some(pos) => line_ir
                                .underlines
                                .push(UnderlineRange { start: pos, end: 0 }),
                            None => underline_pos = Some(column_offset + pulse_ir.columns.len()),
                        }
                    }
                }

                if token.value.is_beat_divider() {
                    if let Some(last) = pulse_ir.columns.last_mut() {
                        last.duration += current_acc;

                        if last_acc < current_acc {
                            last.duration -= last_acc;
                        }
                    }

                    if current_acc == 1. || stream.peek().is_none() {
                        pulse_ir.add_column(PulseColumnKind::EmptyNote);
                    }
                }
            }

            if pulse.tokens.is_empty() {
                pulse_ir.add_column(PulseColumnKind::EmptyNote);
            }

            if let Some(last) = pulse_ir.columns.last_mut() {
                last.duration = current_acc;
            }

            let duration = pulse_ir.columns.iter().map(|c| c.duration).sum::<f32>();

            if duration != 1. {
                let range = self.tree.get_scope_range(pulse.sid);
                self.report_error(range, DiagnosticKind::InvalidNoteDistribution);
            }

            column_offset += pulse_ir.columns.len();
            line_ir.pulses.push(pulse_ir);
        }

        if let Some(sid) = underline_sid
            && underline_pos.is_some()
        {
            let underline_range = self.tree.get_symbol_range(sid);
            let line_range = self.tree.get_scope_range(line.sid);

            self.report_error(
                underline_range.merge(line_range),
                DiagnosticKind::UnmatchedUnderline,
            );
        }

        line_ir
    }

    fn resolve_last_note(&self, line_ir: &SolfaLineIR) -> Option<Note> {
        line_ir
            .pulses
            .iter()
            .rev()
            .find_map(|pulse| pulse.columns.last())
            .and_then(|column| match &column.kind {
                PulseColumnKind::Notes(notes) => notes.last().copied(),
                PulseColumnKind::ProlongedNote(note) => Some(*note),
                PulseColumnKind::EmptyNote => None,
            })
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
            lines.push(value);
        }
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

    fn resolve_solfa_line(
        &mut self,
        node: Node<'_>,
        scope_id: ScopeId,
        id: usize,
    ) -> Option<SolfaLine> {
        let voice = self.resolve_solfa_voice(node, id)?;
        let content_node = node.child_by_field_name("content")?;
        let mut pulses = Vec::new();

        for pulse in content_node.children(&mut node.walk()) {
            let scope_id = self
                .tree
                .add_scope(ScopeKind::Pulse, pulse.range(), scope_id.into());

            if let Some(pulse) = self.resolve_pulse(pulse, scope_id) {
                pulses.push(pulse);
            }
        }

        Some(SolfaLine {
            sid: scope_id,
            voice,
            pulses,
        })
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

    fn resolve_pulse(&mut self, node: Node<'_>, scope_id: ScopeId) -> Option<Pulse> {
        let accent_node = node.child_by_field_name("accent")?;
        let tokens_node = node.child_by_field_name("tokens");

        let accent = self.resovle_pulse_accent(accent_node)?;
        let tokens = tokens_node
            .map(|n| self.resolve_pulse_tokens(n, scope_id))
            .unwrap_or_default();

        Some(Pulse {
            sid: scope_id,
            accent,
            tokens,
        })
    }

    fn resolve_pulse_tokens(&mut self, node: Node<'_>, scope_id: ScopeId) -> Vec<PulseToken> {
        let mut tokens = Vec::new();

        for child in node.named_children(&mut node.walk()) {
            if let Some(token) = self.resolve_pulse_token(child, scope_id) {
                tokens.push(token);
            }
        }

        tokens
    }

    fn resovle_pulse_accent(&mut self, node: Node<'_>) -> Option<PulseAccent> {
        match node.kind_id() {
            node_types::STRONG_ACCENT => Some(PulseAccent::Strong),
            node_types::MEDIUM_ACCENT => Some(PulseAccent::Medium),
            node_types::WEAK_ACCENT => Some(PulseAccent::Weak),
            _ => None,
        }
    }

    fn resolve_pulse_token(&mut self, node: Node<'_>, scope_id: ScopeId) -> Option<PulseToken> {
        let kind = match node.kind_id() {
            node_types::HALF_DIVISION => PulseTokenKind::HalfDivision,
            node_types::QUARTER_DIVISION => PulseTokenKind::QuarterDivision,
            node_types::UNDERLINE_MARKER => PulseTokenKind::UnderlineMarker,
            node_types::NOTE => self.parse_node(node).map(PulseTokenKind::Note)?,
            node_types::PROLONGED_NOTE => PulseTokenKind::ProlongedNote,
            _ => return None,
        };

        let sid = self
            .tree
            .add_symbol(SymbolKind::Token, node.range(), scope_id);

        Some(PulseToken { sid, value: kind })
    }

    fn resolve_section(&mut self, node: Node<'_>, parent_sid: ScopeId) -> Section {
        let sid = self
            .tree
            .add_scope(ScopeKind::Section, node.range(), parent_sid.into());

        let mut section = Section::new(sid);

        for child in node.named_children(&mut node.walk()) {
            match child.kind_id() {
                node_types::PARAMETER_LINE => {
                    self.handle_assignment_node(child, sid, &mut section.params)
                }
                node_types::DYNAMICS_LINE => {
                    self.handle_assignment_node(child, sid, &mut section.dynamics)
                }
                node_types::SOLFA_LINE => self.handle_solfa_node(child, sid, &mut section.solfa),
                node_types::LYRIC_LINE => self.handle_lyric_node(child, sid, &mut section),
                _ => {}
            }
        }

        self.validate_voice_count(&section, node.range());

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
        let anchor = self.resolve_lyric_anchor(anchor_node, &tokens, scope_id);

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
            match child.kind_id() {
                node_types::LYRIC_COLUMN => {
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

        let span = node
            .child_by_field_name("span")
            .and_then(|s| s.child_by_field_name("count"))
            .and_then(|c| self.parse_node(c))
            .unwrap_or(1);

        LyricColumn { span, chunks }
    }

    fn resolve_lyric_atom(&mut self, node: Node<'_>, scope_id: ScopeId) -> Option<LyricChunk> {
        let value = match node.kind_id() {
            node_types::SPACE_OPERATOR => LyricChunkKind::Space,
            node_types::CONCAT_OPERATOR => LyricChunkKind::Concat,
            node_types::NEWLINE_OPERATOR => LyricChunkKind::Newline,
            node_types::UNDERLINE_MARKER => LyricChunkKind::UnderlineMarker,
            node_types::LYRIC_PLACEHOLDER => LyricChunkKind::Placeholder,
            node_types::LYRIC_STRING => LyricChunkKind::String(self.resolve_node_string(node)?),
            _ => {
                let s = self.resolve_node_string(node)?;
                let char = LyricSpecialChar::try_from(s.as_str()).ok()?;
                LyricChunkKind::SpecialChar(char)
            }
        };

        let sid = self
            .tree
            .add_symbol(SymbolKind::Token, node.range(), scope_id);

        Some(LyricChunk { sid, value })
    }

    fn resolve_lyric_anchor(
        &mut self,
        node: Option<Node<'_>>,
        tokens: &[LyricToken],
        scope_id: ScopeId,
    ) -> Option<LyricAnchor> {
        let last_token = tokens.last();

        if let Some(LyricToken::Operator(operator)) = last_token {
            if let Some(node) = node {
                let sid = self
                    .tree
                    .add_symbol(SymbolKind::Token, node.range(), scope_id);

                return Some(LyricAnchor {
                    sid,
                    value: operator.value,
                });
            } else {
                let range = self.tree.get_symbol_range(operator.sid);

                if !matches!(operator.value, LyricOperatorKind::Space) {
                    self.report_error(range, DiagnosticKind::ExpectedLyricAnchor);
                }
            }
        }

        None
    }

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

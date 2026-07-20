use tree_sitter::{Node, QueryCursor, StreamingIterator};

use crate::{
    ast::{
        body::{Body, Section},
        document::Document,
        header::Header,
        lyrics::{
            LyricAnchor, LyricChunk, LyricChunkKind, LyricColumn, LyricLine, LyricOperator,
            LyricOperatorKind, LyricSpecialChar, LyricString, LyricStringKind, LyricToken,
        },
        solfa::{Note, Pulse, PulseAccent, PulseToken, PulseTokenKind, SolfaLine},
        symbols::{
            Comment, Field, FieldAssign, ScopeId, ScopeKind, SymbolId, SymbolKind, SymbolRef,
            SymbolTree,
        },
        types::Voice,
    },
    diagnostic::{Diagnostic, DiagnosticKind, DiagnosticLevel},
    ir::{
        DocumentIR, PulseView, SectionGroup, SectionIR,
        lyrics::{LyricColumnIR, LyricLineIR, LyricStringIR},
        solfa::{PulseColumnKind, PulseIR, SolfaLineIR},
        utils::{BeatBuffer, UnderlineBuffer},
    },
    output::ValidatorOutput,
    ts_utils::{
        context::TSContext,
        generated::node_types,
        parsing::ParseNode,
        range::{Range, RangeMerge},
        types::AssignmentData,
    },
};

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

            self.handle_root_node(root);
            self.handle_query(root, context);
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
                let section_ir = self.build_section_ir(&section);

                body.sections.push(section);
                self.ir.sections.push(section_ir);
            }
        }

        self.document.body = body;
    }

    fn build_section_ir(&mut self, section: &Section) -> SectionIR {
        let mut section_ir = SectionIR::default();

        for line in &section.solfa {
            let line_ir = self.build_solfa_line_ir(line);
            section_ir.solfa.push(line_ir);
        }

        for line in &section.lyrics {
            let line_ir = self.build_lyric_line_ir(line);
            section_ir.lyrics.push(line_ir);
        }

        section_ir.groups = self.build_section_group(section, &section_ir);

        self.validate_section_ir(&section_ir);

        section_ir
    }

    fn build_solfa_line_ir(&mut self, line: &SolfaLine) -> SolfaLineIR {
        let mut line_ir = SolfaLineIR::new(line.sid, line.voice);
        let mut underline_buffer = UnderlineBuffer::default();

        for pulse in &line.pulses {
            let mut stream = pulse.tokens.iter().peekable();
            let mut pulse_ir = PulseIR::new(pulse.sid, pulse.accent.value);
            let mut beat_buffer = BeatBuffer::default();

            if stream.peek().is_none() || stream.peek().is_some_and(|t| t.value.is_beat_divider()) {
                pulse_ir.add_column(PulseColumnKind::EmptyNote);
                beat_buffer.append_note();
            }

            while let Some(token) = stream.next() {
                if token.value.is_beat_divider()
                    && (pulse_ir.columns.is_empty() || stream.peek().is_none())
                {
                    pulse_ir.add_column(PulseColumnKind::EmptyNote);
                    beat_buffer.append_note();
                    break;
                } else if token.value.is_note() {
                    beat_buffer.append_note();
                }

                match &token.value {
                    PulseTokenKind::ProlongedNote => {
                        if let Some(last_note) = self.resolve_last_note(&line_ir) {
                            pulse_ir.add_column(PulseColumnKind::ProlongedNote(last_note));
                        } else {
                            let range = self.tree.get_symbol_range(token.sid);
                            self.report_error(range, DiagnosticKind::InvalidNoteProlongation);
                        }
                    }
                    PulseTokenKind::Note(note) => {
                        pulse_ir.add_column(PulseColumnKind::Note(*note));
                    }
                    PulseTokenKind::HalfDivision => {
                        beat_buffer.divide();
                    }
                    PulseTokenKind::QuarterDivision => {
                        beat_buffer.divide_sub();
                    }
                    PulseTokenKind::UnderlineMarker => {
                        underline_buffer.mark(token.sid, pulse_ir.columns.len());
                    }
                }
            }

            let (durations, length) = beat_buffer.get_durations();

            if !beat_buffer.is_valid() {
                let range = self.tree.get_scope_range(pulse.sid);
                self.report_error(range, DiagnosticKind::InvalidNoteDistribution);
            }

            pulse_ir.set_length(length);
            pulse_ir.fit_durations(&durations);
            underline_buffer.add_offset(pulse_ir.columns.len());

            line_ir.pulses.push(pulse_ir);
        }

        if let Some(sid) = underline_buffer.get_trailing() {
            self.report_trailing_underline(sid, line.sid);
        }

        line_ir.fit_underlines(underline_buffer.results());

        line_ir
    }

    fn resolve_last_note(&self, line_ir: &SolfaLineIR) -> Option<Note> {
        line_ir
            .pulses
            .iter()
            .rev()
            .find_map(|pulse| pulse.columns.last())
            .and_then(|column| match &column.kind {
                PulseColumnKind::Note(note) => Some(*note),
                PulseColumnKind::ProlongedNote(note) => Some(*note),
                PulseColumnKind::EmptyNote => None,
            })
    }

    fn build_lyric_line_ir(&mut self, line: &LyricLine) -> LyricLineIR {
        let mut line_ir = LyricLineIR::new(line.sid);
        let mut underline_buffer = UnderlineBuffer::default();

        for token in &line.tokens {
            match token {
                LyricToken::Column(column) => {
                    let column_ir = self.build_lyric_column_ir(column, &mut underline_buffer);
                    line_ir.columns.push(column_ir);
                }
                LyricToken::Operator(operator) => {
                    line_ir.operators.push(operator.value);
                }
            };
        }

        if let Some(sid) = underline_buffer.get_trailing() {
            self.report_trailing_underline(sid, line.sid);
        }

        line_ir.fit_underlines(underline_buffer.results());

        line_ir
    }

    fn build_lyric_column_ir(
        &mut self,
        column: &LyricColumn,
        underline_buffer: &mut UnderlineBuffer,
    ) -> LyricColumnIR {
        let mut column_ir = LyricColumnIR::new(column.sid, column.span);

        for chunk in &column.chunks {
            match &chunk.value {
                LyricChunkKind::Space => column_ir.operators.push(LyricOperatorKind::Space),
                LyricChunkKind::Newline => column_ir.operators.push(LyricOperatorKind::Newline),
                LyricChunkKind::Placeholder => column_ir.add_chunk(Vec::new()), // placehodler
                LyricChunkKind::String(tokens) => {
                    let lyric_ir = self.build_lyric_string_ir(tokens, underline_buffer);
                    column_ir.add_chunk(lyric_ir);
                }
            }
        }

        column_ir
    }

    fn build_lyric_string_ir(
        &mut self,
        chunks: &[LyricString],
        underline_buffer: &mut UnderlineBuffer,
    ) -> Vec<LyricStringIR> {
        let mut partials = Vec::new();

        for token in chunks {
            match token.value {
                LyricStringKind::UnderlineMarker => {
                    underline_buffer.mark(token.sid, partials.len());
                }
                LyricStringKind::Reference(id) => partials.push(LyricStringIR::Reference(id)),
                LyricStringKind::SpecialChar(ch) => partials.push(LyricStringIR::Special(ch)),
            }
        }

        underline_buffer.add_offset(partials.len());

        partials
    }

    fn build_section_group(
        &mut self,
        section: &Section,
        section_ir: &SectionIR,
    ) -> Vec<SectionGroup> {
        if section.lyrics.is_empty() {
            let solfa = (0..section.solfa.len()).collect::<Vec<_>>();
            let views = self.build_pulse_view(&section_ir.solfa, &solfa);

            return vec![SectionGroup {
                solfa,
                views,
                ..Default::default()
            }];
        }

        let mut groups: Vec<SectionGroup> = Vec::with_capacity(section.lyrics.len());
        let mut last_voice = 0;

        for (idx, lyric) in section.lyrics.iter().enumerate() {
            if last_voice != 0 && lyric.position == last_voice - 1 {
                if let Some(group) = groups.last_mut() {
                    group.lyrics.push(idx);
                }
            } else if !section.solfa.is_empty() {
                let solfa = (last_voice..=lyric.position).collect::<Vec<_>>();
                let views = self.build_pulse_view(&section_ir.solfa, &solfa);

                groups.push(SectionGroup {
                    lyrics: vec![idx],
                    views,
                    solfa,
                });

                last_voice = lyric.position + 1;
            }
        }

        groups
    }

    fn build_pulse_view(&mut self, solfa: &[SolfaLineIR], group: &[usize]) -> Vec<PulseView> {
        let mut views = group
            .first()
            .map(|idx| {
                solfa[*idx]
                    .pulses
                    .iter()
                    .map(PulseView::new)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        for pulse_idx in 0..views.len() {
            for solfa_idx in group.iter().skip(1) {
                let current = &solfa[*solfa_idx];
                let pulse = current.pulses.get(pulse_idx);
                let view = &mut views[pulse_idx];

                if let Some(pulse) = pulse {
                    view.add(pulse);

                    if !view.aligned {
                        break;
                    }
                }
            }
        }

        views
    }

    fn validate_section_ir(&mut self, section_ir: &SectionIR) {
        for group in &section_ir.groups {
            for lyric_idx in &group.lyrics {
                let lyric_line = &section_ir.lyrics[*lyric_idx];
                let mut span_counter = group.width();

                for lyric_col in &lyric_line.columns {
                    if span_counter >= lyric_col.span {
                        span_counter -= lyric_col.span;
                    } else {
                        let range = self.tree.get_scope_range(lyric_col.sid);

                        let ctx_ranges = group
                            .solfa
                            .iter()
                            .map(|idx| self.tree.get_scope_range(section_ir.solfa[*idx].sid))
                            .collect();

                        self.report_error(range, DiagnosticKind::TrailingLyric(ctx_ranges));
                    }
                }
            }
        }
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

        let accent = self.resovle_pulse_accent(accent_node, scope_id)?;
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

    fn resovle_pulse_accent(
        &mut self,
        node: Node<'_>,
        scope_id: ScopeId,
    ) -> Option<SymbolRef<PulseAccent>> {
        let value = match node.kind_id() {
            node_types::STRONG_ACCENT => PulseAccent::Strong,
            node_types::MEDIUM_ACCENT => PulseAccent::Medium,
            node_types::WEAK_ACCENT => PulseAccent::Weak,
            _ => return None,
        };
        let sid = self
            .tree
            .add_symbol(SymbolKind::Token, node.range(), scope_id);

        Some(SymbolRef { sid, value })
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

        self.validate_pulses(&section);
        self.validate_voice_count(&section);

        section
    }

    fn validate_pulses(&mut self, section: &Section) {
        let time_signature = self.document.time_signature().cloned();

        for line in &section.solfa {
            if let Some(first) = section.solfa.first() {
                let first_len = first.pulses.len();
                let current_len = line.pulses.len();

                if current_len != first_len {
                    let range = self.tree.get_scope_range(line.sid);
                    let context_range = self.tree.get_scope_range(first.sid);

                    self.report_error(
                        range,
                        DiagnosticKind::PulseCountMismatch(first_len, current_len, context_range),
                    );
                }
            }

            if let Some(time_signature) = &time_signature {
                let pulse_len = line.pulses.len();
                let mut count = 0;
                let mut offset = 0;

                while count < pulse_len {
                    if count == 0 && offset == pulse_len {
                        break;
                    }

                    let pulse = &line.pulses[offset % pulse_len];

                    offset += 1;

                    if count == 0 && pulse.accent.value != PulseAccent::Strong {
                        continue;
                    }

                    let position = count % time_signature.value.top;
                    let expected = time_signature.value.get_accent(position);

                    if pulse.accent.value != expected {
                        let range = self.tree.get_symbol_range(pulse.accent.sid);
                        let context_range = self.tree.get_symbol_range(time_signature.sid);

                        self.report_error(
                            range,
                            DiagnosticKind::MismatchedPulseAccent(
                                expected,
                                pulse.accent.value,
                                context_range,
                            ),
                        );
                    }

                    count += 1;
                }

                let measure_columns = count % time_signature.value.top;

                if measure_columns != 0 {
                    let measure_start = &line.pulses[pulse_len - measure_columns];
                    let measure_end = &line.pulses[pulse_len - 1];
                    let start_range = self.tree.get_scope_range(measure_start.sid);
                    let end_range = self.tree.get_scope_range(measure_end.sid);
                    let context_range = self.tree.get_symbol_range(time_signature.sid);

                    self.report_error(
                        start_range.merge(end_range),
                        DiagnosticKind::MeasureColumnMismatch(
                            time_signature.value.top,
                            measure_columns,
                            context_range,
                        ),
                    );
                }
            }
        }
    }

    fn validate_voice_count(&mut self, section: &Section) {
        let Some(voices) = &self.document.header.params.voices else {
            return;
        };

        let range = self.tree.get_scope_range(section.sid);
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

        let group = section.solfa.len().saturating_sub(1);
        let verse = self.parse_node(verse_node)?;
        let expected_verse = section
            .lyrics
            .iter()
            .filter(|l| l.position == group)
            .count()
            + 1;
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
            position: group,
            anchor,
            tokens,
        })
    }

    fn resolve_lyric_tokens(&mut self, node: Node<'_>, scope_id: ScopeId) -> Vec<LyricToken> {
        let mut tokens = Vec::new();

        for child in node.named_children(&mut node.walk()) {
            match child.kind_id() {
                node_types::LYRIC_COLUMN => {
                    if let Some(column) = self.resolve_lyric_column(child, scope_id) {
                        tokens.push(LyricToken::Column(column));
                    }
                }
                _ => {
                    if let Some(operator) = self.resolve_lyric_operator(child, scope_id) {
                        if tokens.last().is_some_and(|t| t.is_operator()) {
                            let range = self.tree.get_symbol_range(operator.sid);
                            self.report_error(range, DiagnosticKind::SyntaxError);
                        }

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
        let value = match node.kind_id() {
            node_types::SPACE_OPERATOR => LyricOperatorKind::Space,
            node_types::CONCAT_OPERATOR => LyricOperatorKind::Concat,
            node_types::NEWLINE_OPERATOR => LyricOperatorKind::Newline,
            _ => return None,
        };

        let sid = self
            .tree
            .add_symbol(SymbolKind::Token, node.range(), scope_id);

        Some(LyricOperator { sid, value })
    }

    fn resolve_lyric_column(&mut self, node: Node<'_>, scope_id: ScopeId) -> Option<LyricColumn> {
        let sid = self
            .tree
            .add_scope(ScopeKind::LyricsColumn, node.range(), Some(scope_id));
        let lyric_node = node.child_by_field_name("lyric")?;

        let chunks = match lyric_node.kind_id() {
            node_types::LYRIC_GROUP => lyric_node
                .named_children(&mut node.walk())
                .filter_map(|c| self.resolve_lyric_chunk(c, scope_id))
                .collect(),
            _ => {
                vec![self.resolve_lyric_chunk(lyric_node, scope_id)?]
            }
        };

        let span = node
            .child_by_field_name("span")
            .and_then(|s| s.child_by_field_name("count"))
            .and_then(|c| self.parse_node(c))
            .unwrap_or(1);

        Some(LyricColumn { sid, span, chunks })
    }

    fn resolve_lyric_chunk(&mut self, node: Node<'_>, scope_id: ScopeId) -> Option<LyricChunk> {
        let value = match node.kind_id() {
            node_types::SPACE_OPERATOR => LyricChunkKind::Space,
            node_types::NEWLINE_OPERATOR => LyricChunkKind::Newline,
            node_types::LYRIC_PLACEHOLDER => LyricChunkKind::Placeholder,

            node_types::LYRIC_CHUNK => {
                let scope_id =
                    self.tree
                        .add_scope(ScopeKind::LyricString, node.range(), Some(scope_id));

                LyricChunkKind::String(
                    node.named_children(&mut node.walk())
                        .filter_map(|c| self.resolve_lyric_string(c, scope_id))
                        .collect(),
                )
            }
            _ => return None,
        };

        let sid = self
            .tree
            .add_symbol(SymbolKind::Token, node.range(), scope_id);

        Some(LyricChunk { sid, value })
    }

    fn resolve_lyric_string(&mut self, node: Node<'_>, scope_id: ScopeId) -> Option<LyricString> {
        let sid = self
            .tree
            .add_symbol(SymbolKind::Token, node.range(), scope_id);

        let value = match node.kind_id() {
            node_types::UNDERLINE_MARKER => LyricStringKind::UnderlineMarker,
            node_types::LYRIC_STRING => {
                let chunk = self.resolve_node_string(node)?;
                let id = self.tree.store_lyric_chunk(chunk);
                LyricStringKind::Reference(id)
            }
            node_types::LYRIC_SPECIAL => {
                let s = self.resolve_node_string(node)?;
                let char = LyricSpecialChar::try_from(s.as_str()).ok()?;
                LyricStringKind::SpecialChar(char)
            }
            _ => return None,
        };

        Some(LyricString { sid, value })
    }

    fn resolve_lyric_anchor(
        &mut self,
        node: Option<Node<'_>>,
        tokens: &[LyricToken],
        scope_id: ScopeId,
    ) -> Option<LyricAnchor> {
        let last_token = tokens.last();

        if let Some(LyricToken::Operator(operator)) = last_token {
            if let Some(node) = node
                && node.kind_id() == node_types::LYRIC_ANCHOR
            {
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

    fn handle_query(&mut self, root: Node<'_>, context: &mut TSContext) {
        let capture_names = context.query.capture_names();

        let mut cursor = QueryCursor::new();
        let mut matches = cursor.matches(&context.query, root, self.source);

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
            "comment" | "directive" => {
                self.handle_comment_node(node);
            }
            "error.syntax" => {
                self.report_error(node.range(), DiagnosticKind::SyntaxError);
            }
            "error.missing" => {
                self.report_error(node.range(), DiagnosticKind::Missing(node.kind().into()));
            }
            _ => {}
        }
    }

    fn handle_comment_node(&mut self, node: Node<'_>) {
        if let Some(comment) = self.resolve_node_string(node) {
            let sid = self.tree.add_symbol(SymbolKind::Comment, node.range(), 0);
            let value = comment.trim().to_string();

            self.tree.comments.push(Comment { sid, value });
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

    fn report_trailing_underline(&mut self, underline_sid: SymbolId, line_sid: ScopeId) {
        let underline_range = self.tree.get_symbol_range(underline_sid);
        let line_range = self.tree.get_scope_range(line_sid);

        self.report_error(
            underline_range.merge(line_range),
            DiagnosticKind::UnmatchedUnderline,
        );
    }
}

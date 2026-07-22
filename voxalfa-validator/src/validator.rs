use tree_sitter::{Node, QueryCursor, StreamingIterator};

use crate::{
    ast::{
        body::{Section, SubSection},
        header::Header,
        lyrics::{
            LyricChunk, LyricChunkKind, LyricColumn, LyricLine, LyricOperator, LyricOperatorKind,
            LyricSpecialChar, LyricString, LyricStringKind, LyricToken,
        },
        solfa::{Note, Pulse, PulseAccent, PulseToken, PulseTokenKind, SolfaLine},
        symbols::{
            Comment, Field, FieldAssign, ScopeId, ScopeKind, SymbolId, SymbolKind, SymbolRef,
            SymbolTree,
        },
        types::{TimeSignature, Voice},
    },
    diagnostic::{Diagnostic, DiagnosticKind, DiagnosticLevel},
    ir::{
        DocumentIR, PulseView, SectionIR, SubSectionIR,
        lyrics::{LyricColumnIR, LyricLineIR, LyricStringIR},
        solfa::{PulseColumnKind, PulseIR, SolfaLineIR},
        utils::{BeatBuffer, UnderlineBuffer},
    },
    output::ValidatorOutput,
    ts_utils::{
        context::TSContext,
        generated::node_types,
        parsing::ParseNode,
        range::{Range, RangeUtil},
        types::AssignmentData,
    },
};

#[derive(Debug)]
pub struct DocumentValidator<'a> {
    pub source: &'a [u8],
    pub tree: SymbolTree,
    pub header: Header,
    pub ir: DocumentIR,
    pub diagnostics: Vec<Diagnostic>,
}

impl<'a> DocumentValidator<'a> {
    pub fn new(source: &'a str) -> Self {
        Self {
            source: source.as_bytes(),
            header: Header::default(),
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
            header: self.header,
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

        self.header = header;
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
        let body_sid = self.tree.add_scope(ScopeKind::Body, node.range(), None);
        let mut sections = Vec::new();

        for child in node.named_children(&mut node.walk()) {
            if child.kind_id() == node_types::SECTION {
                let section = self.resolve_section(child, body_sid);

                self.validate_pulses(&section);
                self.validate_voices(&section);

                sections.push(section);
            }
        }

        self.validate_lyrics_join(&sections);

        // strip away unused data and transform raw token streams into structured data
        let sections_ir = sections
            .into_iter()
            .map(|s| self.build_section_ir(s))
            .collect::<Vec<_>>();

        self.validate_sections(&sections_ir);

        self.ir.sections = sections_ir;
    }

    fn resolve_section(&mut self, node: Node<'_>, parent_sid: ScopeId) -> Section {
        let sid = self
            .tree
            .add_scope(ScopeKind::Section, node.range(), parent_sid.into());

        let mut section = Section::new(sid);

        if let Some(prev) = node.prev_sibling() {
            if prev.kind_id() == node_types::SECTION_MERGE {
                section.merge = true;
            }
        }

        for child in node.named_children(&mut node.walk()) {
            if child.kind_id() == node_types::SUB_SECTION {
                self.handle_sub_section_node(child, sid, &mut section);
            }
        }

        section
    }

    fn build_section_ir(&mut self, section: Section) -> SectionIR {
        let blocks = section
            .items
            .into_iter()
            .map(|s| self.build_sub_section_ir(s))
            .collect::<Vec<_>>();

        SectionIR {
            sid: section.sid,
            params: section.params,
            merge: section.merge,
            items: blocks,
        }
    }

    fn build_sub_section_ir(&mut self, section: SubSection) -> SubSectionIR {
        let solfa = section
            .solfa
            .into_iter()
            .map(|s| self.build_solfa_line_ir(s))
            .collect::<Vec<_>>();

        let lyrics = section
            .lyrics
            .into_iter()
            .map(|l| self.build_lyric_line_ir(l))
            .collect();

        let views = self.build_pulse_view(&solfa);

        SubSectionIR {
            sid: section.sid,
            dynamics: section.dynamics,
            views,
            solfa,
            lyrics,
        }
    }

    fn build_solfa_line_ir(&mut self, line: SolfaLine) -> SolfaLineIR {
        let mut line_ir = SolfaLineIR::new(line.sid, line.voice.value);
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

    fn build_lyric_line_ir(&mut self, line: LyricLine) -> LyricLineIR {
        let mut line_ir = LyricLineIR::new(&line);
        let mut underline_buffer = UnderlineBuffer::default();

        for token in line.tokens {
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
        column: LyricColumn,
        underline_buffer: &mut UnderlineBuffer,
    ) -> LyricColumnIR {
        let mut column_ir = LyricColumnIR::new(column.sid, column.span);

        for chunk in column.chunks {
            match chunk.value {
                LyricChunkKind::Space => column_ir.operators.push(LyricOperatorKind::Space),
                LyricChunkKind::Newline => column_ir.operators.push(LyricOperatorKind::Newline),
                LyricChunkKind::Placeholder => column_ir.placeholder = true,
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
        chunks: Vec<LyricString>,
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

    fn build_pulse_view(&mut self, solfa: &[SolfaLineIR]) -> Vec<PulseView> {
        let mut views = solfa
            .first()
            .map(|first| first.pulses.iter().map(PulseView::new).collect::<Vec<_>>())
            .unwrap_or_default();

        for (pulse_idx, view) in views.iter_mut().enumerate() {
            for current in solfa.iter().skip(1) {
                let pulse = current.pulses.get(pulse_idx);

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

    fn validte_section(&mut self, section: &SectionIR) {
        for sub_section in &section.items {
            self.validate_sub_section_ir(sub_section);
        }
    }

    fn validate_sub_section_ir(&mut self, sub_section: &SubSectionIR) {
        let range = self.tree.get_scope_range(sub_section.sid);

        if let Some(verses) = &self.header.metadata.verses {
            let value = sub_section.lyrics.len();

            if value != verses.value {
                let context_range = self.tree.get_symbol_range(verses.sid);

                self.report_error(
                    range,
                    DiagnosticKind::VerseMismatch(verses.value, value, context_range),
                );
            }
        } else if !sub_section.lyrics.is_empty() {
            let context_range = self.tree.get_scope_range(self.header.sid);

            self.report_error(
                range,
                DiagnosticKind::UndefinedVersesMetadata(context_range),
            );
        }

        for lyric_line in &sub_section.lyrics {
            let mut span_counter = sub_section.width();

            for lyric_col in &lyric_line.columns {
                if span_counter >= lyric_col.span {
                    span_counter -= lyric_col.span;
                } else {
                    let range = self.tree.get_scope_range(lyric_col.sid);

                    let context_ranges = sub_section
                        .solfa
                        .iter()
                        .map(|s| self.tree.get_scope_range(s.sid))
                        .collect();

                    self.report_error(range, DiagnosticKind::TrailingLyric(context_ranges));
                }
            }
        }
    }

    fn handle_solfa_node(
        &mut self,
        node: Node<'_>,
        parent_sid: ScopeId,
        sub_section: &mut SubSection,
    ) {
        let sid = self
            .tree
            .add_scope(ScopeKind::SolfaLine, node.range(), parent_sid.into());

        if let Some(value) = self.resolve_solfa_line(node, sid) {
            sub_section.solfa.push(value);
        }
    }

    fn handle_lyric_node(&mut self, node: Node<'_>, parent_sid: ScopeId, section: &mut SubSection) {
        let sid = self
            .tree
            .add_scope(ScopeKind::LyricLine, node.range(), parent_sid.into());

        if let Some(line) = self.resolve_lyric_line(node, sid, section) {
            section.lyrics.push(line);
        }
    }

    fn resolve_solfa_line(&mut self, node: Node<'_>, scope_id: ScopeId) -> Option<SolfaLine> {
        let voice = self.resolve_solfa_voice(node, scope_id)?;
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

    fn resolve_solfa_voice(
        &mut self,
        node: Node<'_>,
        parent_sid: ScopeId,
    ) -> Option<SymbolRef<Voice>> {
        let voice_node = node.child_by_field_name("voice")?;
        let voice_str = self.resolve_node_string(voice_node)?;
        let voice = Voice::try_from(voice_str.as_str());

        let sid = self
            .tree
            .add_symbol(SymbolKind::Token, voice_node.range(), parent_sid);

        if let Ok(value) = voice {
            Some(SymbolRef { sid, value })
        } else {
            self.report_error(voice_node.range(), DiagnosticKind::InvalidVoice(voice_str));
            None
        }
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

    fn handle_sub_section_node(
        &mut self,
        node: Node<'_>,
        parent_sid: ScopeId,
        section: &mut Section,
    ) {
        let sid = self
            .tree
            .add_scope(ScopeKind::SubSection, node.range(), parent_sid.into());

        let mut result = SubSection::new(sid);

        for child in node.named_children(&mut node.walk()) {
            match child.kind_id() {
                node_types::PARAMETER_LINE => {
                    self.handle_sction_param_node(child, parent_sid, section);
                }
                node_types::DYNAMICS_LINE => {
                    self.handle_assignment_node(child, sid, &mut result.dynamics)
                }
                node_types::SOLFA_LINE => self.handle_solfa_node(child, sid, &mut result),
                node_types::LYRIC_LINE => self.handle_lyric_node(child, sid, &mut result),
                _ => {}
            }
        }

        section.items.push(result);
    }

    fn handle_sction_param_node(
        &mut self,
        node: Node<'_>,
        section_sid: ScopeId,
        section: &mut Section,
    ) {
        if !section.items.is_empty() {
            let context_range = self.tree.get_scope_range(section.sid).start();

            self.report_error(
                node.range(),
                DiagnosticKind::NonTopLevelParamsOverride(context_range),
            );
        }

        self.handle_assignment_node(node, section_sid, &mut section.params)
    }

    fn validate_pulses(&mut self, section: &Section) {
        let time_signature = self.header.params.time.clone();

        let solfa = section
            .items
            .iter()
            .flat_map(|s| &s.solfa)
            .collect::<Vec<_>>();

        for line in solfa.iter().skip(1) {
            if let Some(first) = solfa.first() {
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
                self.validate_time_signature(line, time_signature);
            } else {
                let range = self.tree.get_scope_range(line.sid);
                let context_range = self.tree.get_scope_range(self.header.sid);

                self.report_error(range, DiagnosticKind::UndefinedTimeParameter(context_range));
            }
        }
    }

    fn validate_time_signature(
        &mut self,
        line: &SolfaLine,
        time_signature: &SymbolRef<TimeSignature>,
    ) {
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

    fn validate_voices(&mut self, section: &Section) {
        let Some(voices) = &self.header.metadata.voices else {
            return;
        };

        let range = self.tree.get_scope_range(section.sid);
        let expected_len = voices.value.len();
        let context_range = self.tree.get_symbol_range(voices.sid);

        let voices = section
            .items
            .iter()
            .flat_map(|sub| sub.solfa.iter().map(|s| &s.voice))
            .collect::<Vec<_>>();

        if voices.len() != expected_len {
            self.report_error(
                range,
                DiagnosticKind::VoiceCountMismatch(expected_len, voices.len(), context_range),
            );
        }

        for (id, voice) in voices.iter().enumerate() {
            let range = self.tree.get_symbol_range(voice.sid);

            if let Some(voices) = &self.header.metadata.voices {
                if let Some(expected_voice) = voices.value.get(id) {
                    if voice.value != *expected_voice {
                        self.report_error(
                            range,
                            DiagnosticKind::VoiceMismatch(*expected_voice, voice.value),
                        );
                    }
                } else {
                    let context_range = self.tree.get_symbol_range(voices.sid);

                    self.report_error(
                        range,
                        DiagnosticKind::UndefinedVoice(voice.value, context_range),
                    );
                }
            } else {
                let context_range = self.tree.get_scope_range(self.header.sid);

                self.report_error(range, DiagnosticKind::UndefinedVoiceMetadata(context_range));
            }
        }
    }

    fn validate_lyrics_join(&mut self, sections: &[Section]) {
        for (id, section) in sections.iter().enumerate() {
            if let Some(next_section) = sections.get(id + 1) {
                for line in section.items.iter().flat_map(|s| &s.lyrics) {
                    if line.anchor.is_none() {
                        let range = self.tree.get_scope_range(line.sid);
                        let context_range = self.tree.get_scope_range(next_section.sid);

                        self.report_error(
                            range.end(),
                            DiagnosticKind::ExpectedLyricJoin(context_range),
                        );
                    }
                }
            } else {
                for line in section.items.iter().flat_map(|s| &s.lyrics) {
                    if let Some((anchor_range, LyricToken::Operator(op))) =
                        line.anchor.zip(line.tokens.last())
                    {
                        let operator_range = self.tree.get_symbol_range(op.sid);
                        let range = operator_range.merge(anchor_range);
                        let context_range = self.tree.get_scope_range(section.sid);

                        self.report_error(
                            range,
                            DiagnosticKind::UnusedLyricJoin(context_range.end()),
                        );
                    }
                }
            }
        }
    }

    fn validate_sections(&mut self, sections: &[SectionIR]) {
        for (section_idx, section) in sections.iter().enumerate() {
            self.validte_section(section);

            if section.merge {
                self.validate_section_merge(section_idx, &sections);
            }

            // TODO: dynamics valdaition
            // for (sub_idx, sub_section) in section.items.iter().enumerate() {
            // }
        }
    }

    fn validate_section_merge(&mut self, section_idx: usize, sections: &[SectionIR]) {
        let current = &sections[section_idx];
        let root = sections[..section_idx].iter().rev().find(|s| !s.merge);

        if let Some(root) = root {
            let current_dist = root.items.iter().map(|sub| sub.solfa.len());
            let target_dist = current.items.iter().map(|sub| sub.solfa.len());

            if !current_dist.eq(target_dist) {
                let range = self.tree.get_scope_range(current.sid);
                let context_range = self.tree.get_scope_range(root.sid);

                self.report_error(range, DiagnosticKind::InvalidSectionMerge(context_range));
            }
        }
    }

    fn resolve_lyric_line(
        &mut self,
        node: Node<'_>,
        scope_id: ScopeId,
        section: &SubSection,
    ) -> Option<LyricLine> {
        let verse_node = node.child_by_field_name("verse")?;
        let content_node = node.child_by_field_name("content")?;
        let anchor_node = node.child_by_field_name("anchor");

        let verse = self.parse_node(verse_node)?;
        let expected_verse = section.lyrics.len() + 1;
        let tokens = self.resolve_lyric_tokens(content_node, scope_id);
        let anchor = self.resolve_lyric_anchor(anchor_node, &tokens);

        if verse != expected_verse {
            self.report_warning(
                verse_node.range(),
                DiagnosticKind::MismatchedVerseIndex(expected_verse, verse),
            );
        }

        Some(LyricLine {
            sid: scope_id,
            verse: expected_verse,
            anchor,
            tokens,
        })
    }

    fn resolve_lyric_tokens(&mut self, node: Node<'_>, scope_id: ScopeId) -> Vec<LyricToken> {
        let mut tokens = Vec::new();

        for child in node.named_children(&mut node.walk()) {
            if child.kind_id() == node_types::LYRIC_COLUMN {
                if let Some(column) = self.resolve_lyric_column(child, scope_id) {
                    tokens.push(LyricToken::Column(column));
                }
            } else if let Some(operator) = self.resolve_lyric_operator(child, scope_id) {
                if tokens.last().is_some_and(|t| t.is_operator())
                    && matches!(operator.value, LyricOperatorKind::Space)
                {

                    // ignore trailing spaces after another opeartor
                } else {
                    tokens.push(LyricToken::Operator(operator));
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
    ) -> Option<Range> {
        let last_token = tokens.last();

        match (node, last_token) {
            (Some(node), Some(LyricToken::Column(_))) => {
                self.report_error(node.range(), DiagnosticKind::SyntaxError)
            }
            (None, Some(LyricToken::Operator(operator))) => {
                if operator.value != LyricOperatorKind::Space {
                    let range = self.tree.get_symbol_range(operator.sid);
                    self.report_error(range, DiagnosticKind::ExpectedLyricAnchor);
                }
            }
            (Some(node), Some(LyricToken::Operator(_))) => {
                return Some(node.range());
            }
            _ => {}
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

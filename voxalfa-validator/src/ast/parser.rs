use tree_sitter::{Node, QueryCursor, StreamingIterator};

use crate::{
    ast::{
        body::{Body, Section, SubSection},
        header::Header,
        lyrics::{
            LyricChunk, LyricChunkKind, LyricColumn, LyricLine, LyricOperator, LyricOperatorKind,
            LyricSpecialChar, LyricString, LyricStringKind, LyricToken,
        },
        solfa::{Pulse, PulseAccent, PulseToken, PulseTokenKind, SolfaLine},
        symbols::{
            Comment, Delimiter, DelimiterKind, Field, FieldAssign, ScopeId, ScopeKind, SymbolKind,
            SymbolRef, SymbolTree,
        },
        types::Voice,
    },
    diagnostics::{
        reporter::DiagnosticReporter,
        types::{DiagnosticKind, ReportStage},
    },
    ts_utils::{
        context::TSContext,
        generated::node_types,
        parsing::ParseNode,
        range::{Range, RangeUtil},
        types::AssignmentData,
    },
};

#[derive(Debug)]
pub struct ParserOutput {
    pub header: Header,
    pub body: Body,
    pub tree: SymbolTree,
    pub reporter: DiagnosticReporter,
    pub delimiters: Vec<Delimiter>,
}

#[derive(Debug)]
pub struct Parser<'a> {
    source: &'a [u8],
    header: Header,
    body: Body,
    delimiters: Vec<Delimiter>,
    pub(crate) tree: SymbolTree,
    pub(crate) reporter: DiagnosticReporter,
}

impl<'a> Parser<'a> {
    pub fn new(source: &'a str) -> Self {
        Self {
            source: source.as_bytes(),
            header: Header::default(),
            body: Body::default(),
            tree: SymbolTree::default(),
            delimiters: Vec::new(),
            reporter: DiagnosticReporter::new(ReportStage::Parsing),
        }
    }

    pub fn parse(mut self, context: &mut TSContext) -> ParserOutput {
        if let Some(tree) = context.parse(self.source) {
            let root = tree.root_node();

            self.handle_root_node(root);
            self.handle_query(root, context);
        }

        ParserOutput {
            header: self.header,
            body: self.body,
            tree: self.tree,
            reporter: self.reporter,
            delimiters: self.delimiters,
        }
    }

    fn handle_root_node(&mut self, root: Node<'_>) {
        for child in root.named_children(&mut root.walk()) {
            match child.kind_id() {
                node_types::HEADER => self.handle_header_node(child),
                node_types::BODY => self.handle_body_node(child),
                node_types::HEADER_DELIMITER => self.add_delimiter(child, DelimiterKind::Header),
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
                    self.handle_local_params_node(child, sid, &mut header.metadata)
                }
                node_types::PARAMETER_LINE => {
                    self.handle_local_params_node(child, sid, &mut header.params)
                }
                _ => {}
            }
        }

        self.header = header;
    }

    fn handle_body_node(&mut self, node: Node<'_>) {
        self.body.sid = self.tree.add_scope(ScopeKind::Body, node.range(), None);

        for child in node.named_children(&mut node.walk()) {
            match child.kind_id() {
                node_types::SECTION_SPLIT_DELIMITER => {
                    self.add_delimiter(child, DelimiterKind::SectionSplit)
                }
                node_types::SECTION_MERGE_DELIMITER => {
                    self.add_delimiter(child, DelimiterKind::SectionMerge)
                }
                node_types::SECTION => self.handle_section_node(child),
                _ => {}
            }
        }
    }

    fn handle_section_node(&mut self, node: Node<'_>) {
        let sid = self
            .tree
            .add_scope(ScopeKind::Section, node.range(), self.body.sid.into());

        let mut section = Section::new(sid);

        if let Some(prev) = node.prev_sibling()
            && prev.kind_id() == node_types::SECTION_MERGE_DELIMITER
        {
            section.merge = true;
        }

        for child in node.named_children(&mut node.walk()) {
            match child.kind_id() {
                node_types::SUB_SECTION => self.handle_sub_section_node(child, sid, &mut section),
                node_types::SUB_SECTION_DELIMITER => {
                    self.add_delimiter(child, DelimiterKind::SubSection)
                }
                _ => {}
            }
        }

        self.body.sections.push(section);
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
                    self.handle_global_params_node(child, parent_sid, section);
                    self.handle_local_params_node(child, sid, &mut result.params)
                }
                node_types::METADATA_LINE => {
                    self.handle_section_metadata_node(child, parent_sid, section);
                }
                node_types::SOLFA_LINE => self.handle_solfa_node(child, sid, &mut result),
                node_types::LYRIC_LINE => self.handle_lyric_node(child, sid, &mut result),
                _ => {}
            }
        }

        section.items.push(result);
    }

    fn handle_global_params_node(
        &mut self,
        node: Node<'_>,
        section_sid: ScopeId,
        section: &mut Section,
    ) {
        if !section.items.is_empty() {
            let context_range = self.tree.get_scope_range(section.sid).start();

            self.reporter.error(
                node.range(),
                DiagnosticKind::NonTopLevelParamsOverride(context_range),
            );
        }

        self.handle_local_params_node(node, section_sid, &mut section.params)
    }

    fn handle_section_metadata_node(
        &mut self,
        node: Node<'_>,
        section_sid: ScopeId,
        section: &mut Section,
    ) {
        if !section.items.is_empty() {
            let context_range = self.tree.get_scope_range(section.sid).start();

            self.reporter.error(
                node.range(),
                DiagnosticKind::NonTopLevelSectionMetadata(context_range),
            );
        }

        self.handle_local_params_node(node, section_sid, &mut section.metadata)
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
        let voice = Voice::try_from(voice_str.clone());

        let sid = self
            .tree
            .add_symbol(SymbolKind::Token, voice_node.range(), parent_sid);

        if let Ok(value) = voice {
            Some(SymbolRef { sid, value })
        } else {
            self.reporter
                .error(voice_node.range(), DiagnosticKind::InvalidVoice(voice_str));
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
            node_types::NOTE => self.parse_node(node, scope_id).map(PulseTokenKind::Note)?,
            node_types::PROLONGED_NOTE => PulseTokenKind::ProlongedNote,
            _ => return None,
        };

        let sid = self
            .tree
            .add_symbol(SymbolKind::Token, node.range(), scope_id);

        Some(PulseToken { sid, value: kind })
    }

    fn handle_lyric_node(&mut self, node: Node<'_>, parent_sid: ScopeId, section: &mut SubSection) {
        let sid = self
            .tree
            .add_scope(ScopeKind::LyricLine, node.range(), parent_sid.into());

        if let Some(line) = self.resolve_lyric_line(node, sid, section) {
            section.lyrics.push(line);
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

        let verse = self.parse_node(verse_node, scope_id)?;
        let expected_verse = section.lyrics.len() + 1;
        let tokens = self.resolve_lyric_tokens(content_node, scope_id);
        let anchor = self.resolve_lyric_anchor(anchor_node, &tokens);

        if verse != expected_verse {
            self.reporter.warning(
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
            .and_then(|c| self.parse_node(c, scope_id))
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
            (Some(node), Some(LyricToken::Column(_))) => self
                .reporter
                .error(node.range(), DiagnosticKind::SyntaxError),
            (None, Some(LyricToken::Operator(operator)))
                if operator.value != LyricOperatorKind::Space =>
            {
                let range = self.tree.get_symbol_range(operator.sid);
                self.reporter
                    .error(range, DiagnosticKind::ExpectedLyricAnchor);
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
                self.reporter
                    .error(node.range(), DiagnosticKind::SyntaxError);
            }
            "error.missing" => {
                self.reporter
                    .error(node.range(), DiagnosticKind::Missing(node.kind().into()));
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

    fn handle_local_params_node<T: FieldAssign>(
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

    fn add_delimiter(&mut self, node: Node<'_>, kind: DelimiterKind) {
        self.delimiters.push(Delimiter {
            kind,
            line: node.range().line(),
        });
    }

    pub(crate) fn parse_node<T: ParseNode>(
        &mut self,
        node: Node<'_>,
        scope_id: ScopeId,
    ) -> Option<T> {
        T::parse_node(self, node, scope_id)
    }

    pub(crate) fn resolve_node_string(&mut self, node: Node<'_>) -> Option<String> {
        node.utf8_text(self.source)
            .map(String::from)
            .inspect_err(|e| {
                self.reporter
                    .error(node.range(), DiagnosticKind::InvalidUTF8(*e))
            })
            .ok()
    }

    pub(crate) fn assign_field<T: ParseNode>(
        &mut self,
        data: AssignmentData,
        field: &mut Field<T>,
    ) {
        if let Some(value) = field {
            let name = data.key_name.clone();
            let scope = self.tree.resolve_scope(value.sid);

            self.reporter.warning(
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
            .parse_node(data.value_node, data.scope_id)
            .map(|value| SymbolRef { sid, value });
    }
}

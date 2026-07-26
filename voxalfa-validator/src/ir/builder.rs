use crate::{
    ast::{
        body::{Body, Section, SubSection},
        lyrics::{
            LyricChunkKind, LyricColumn, LyricLine, LyricOperatorKind, LyricString,
            LyricStringKind, LyricToken,
        },
        solfa::{Note, PulseTokenKind, SolfaLine},
        symbols::{ScopeId, SymbolId, SymbolTree},
    },
    diagnostics::{
        reporter::DiagnosticReporter,
        types::{DiagnosticKind, ReportStage},
    },
    ir::{
        BodyIR, PulseView, SectionIR, SubSectionIR,
        lyrics::{LyricColumnIR, LyricLineIR, LyricStringIR},
        solfa::{PulseColumnKind, PulseIR, SolfaLineIR},
        utils::{BeatBuffer, UnderlineBuffer},
    },
    ts_utils::range::RangeUtil,
};

#[derive(Debug)]
pub struct IRBuilderOutput {
    pub body: BodyIR,
    pub reporter: DiagnosticReporter,
}

#[derive(Debug)]
pub struct IRBuilder<'a> {
    pub tree: &'a SymbolTree,
    pub reporter: DiagnosticReporter,
}

impl<'a> IRBuilder<'a> {
    pub fn new(tree: &'a SymbolTree) -> Self {
        Self {
            tree,
            reporter: DiagnosticReporter::new(ReportStage::IRBuild),
        }
    }

    pub fn build(mut self, body: Body) -> IRBuilderOutput {
        let body = BodyIR {
            sections: body
                .sections
                .into_iter()
                .map(|s| self.build_section_ir(s))
                .collect(),
        };

        IRBuilderOutput {
            body,
            reporter: self.reporter,
        }
    }

    fn build_section_ir(&mut self, section: Section) -> SectionIR {
        let blocks = section
            .items
            .into_iter()
            .map(|s| self.build_sub_section_ir(s))
            .collect::<Vec<_>>();

        SectionIR {
            sid: section.sid,
            merge: section.merge,
            metadata: section.metadata,
            params: section.params,
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
            params: section.params,
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
                            self.reporter
                                .error(range, DiagnosticKind::InvalidNoteProlongation);
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
                self.reporter
                    .error(range, DiagnosticKind::InvalidNoteDistribution);
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

        for chunk in &column.chunks {
            match &chunk.value {
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

    fn build_pulse_view(&mut self, solfa: &[SolfaLineIR]) -> Vec<PulseView> {
        let mut views = solfa
            .first()
            .map(|first| first.pulses.iter().map(PulseView::new).collect::<Vec<_>>())
            .unwrap_or_default();

        for (pulse_id, view) in views.iter_mut().enumerate() {
            for current in solfa.iter().skip(1) {
                let pulse = current.pulses.get(pulse_id);

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

    fn report_trailing_underline(&mut self, underline_sid: SymbolId, line_sid: ScopeId) {
        let underline_range = self.tree.get_symbol_range(underline_sid);
        let line_range = self.tree.get_scope_range(line_sid);

        self.reporter.error(
            underline_range.merge(line_range),
            DiagnosticKind::UnmatchedUnderline,
        );
    }
}

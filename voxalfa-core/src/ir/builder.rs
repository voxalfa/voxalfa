use crate::{
    ast::{
        body::{Body, Section, SubSection},
        lyrics::{
            LyricChunkKind, LyricColumn, LyricLine, LyricOperatorKind, LyricString,
            LyricStringKind, LyricToken,
        },
        solfa::{PulseTokenKind, SolfaLine},
        symbols::{ScopeId, SymbolId, SymbolTree},
    },
    diagnostics::{
        reporter::DiagnosticReporter,
        types::{DiagnosticKind, ReportStage},
    },
    ir::{
        BodyIr, PulseView, SectionIr, SubSectionIr,
        lyrics::{LyricColumnIR, LyricLineIr, LyricStringIR},
        solfa::{NoteKind, PulseColumn, PulseIr, SolfaLineIr},
        utils::{BeatBuffer, UnderlineBuffer, UnderlineMarker},
    },
    ts_utils::range::RangeUtil,
};

#[derive(Debug)]
pub struct IRBuilderOutput {
    pub body: BodyIr,
    pub reporter: DiagnosticReporter,
}

#[derive(Debug)]
pub struct IrBuilder<'a> {
    pub tree: &'a SymbolTree,
    pub reporter: DiagnosticReporter,
}

impl<'a> IrBuilder<'a> {
    pub fn new(tree: &'a SymbolTree) -> Self {
        Self {
            tree,
            reporter: DiagnosticReporter::new(ReportStage::IRBuild),
        }
    }

    pub fn build(mut self, body: Body) -> IRBuilderOutput {
        let body = BodyIr {
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

    fn build_section_ir(&mut self, section: Section) -> SectionIr {
        let blocks = section
            .items
            .into_iter()
            .map(|s| self.build_sub_section_ir(s))
            .collect::<Vec<_>>();

        SectionIr {
            sid: section.sid,
            merge: section.merge,
            params: section.params,
            items: blocks,
        }
    }

    fn build_sub_section_ir(&mut self, section: SubSection) -> SubSectionIr {
        let mut solfa = section
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

        self.expand_empty_notes(&mut solfa);

        SubSectionIr {
            sid: section.sid,
            params: section.params,
            views,
            solfa,
            lyrics,
        }
    }

    fn build_solfa_line_ir(&mut self, line: SolfaLine) -> SolfaLineIr {
        let mut line_ir = SolfaLineIr::new(line.sid, line.voice.value);
        let mut underline_buffer = UnderlineBuffer::default();

        for pulse in &line.pulses {
            let mut stream = pulse.tokens.iter().peekable();
            let mut pulse_ir = PulseIr::new(pulse.sid, pulse.accent.value);
            let mut beat_buffer = BeatBuffer::default();

            if stream.peek().is_none() || stream.peek().is_some_and(|t| t.value.is_beat_divider()) {
                pulse_ir.add_column(NoteKind::EmptyNote);
                beat_buffer.append_note();
            }

            while let Some(token) = stream.next() {
                if token.value.is_beat_divider()
                    && (pulse_ir.columns.is_empty() || stream.peek().is_none())
                {
                    pulse_ir.add_column(NoteKind::EmptyNote);
                    beat_buffer.append_note();
                    break;
                } else if token.value.is_note() {
                    beat_buffer.append_note();
                }

                match &token.value {
                    PulseTokenKind::ProlongedNote => {
                        pulse_ir.add_column(NoteKind::ProlongedNote);
                    }
                    PulseTokenKind::Note(note) => {
                        pulse_ir.add_column(NoteKind::Note(*note));
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

    fn build_lyric_line_ir(&mut self, line: LyricLine) -> LyricLineIr {
        let mut line_ir = LyricLineIr::new(&line);
        let mut underline_buffer = UnderlineBuffer::default();

        for token in line.tokens {
            match token {
                LyricToken::Column(column) => {
                    let column_ir = self.build_lyric_column_ir(column, &mut underline_buffer);
                    line_ir.columns.push(column_ir);
                }
                LyricToken::Operator(operator) => {
                    line_ir.operators.push(operator);
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

    fn expand_empty_notes(&self, solfa: &mut [SolfaLineIr]) {
        let Some(first) = solfa.first() else { return };

        for pulse_id in 0..first.pulses.len() {
            let col_shape = solfa
                .iter()
                .flat_map(|line| line.pulses.get(pulse_id))
                .map(|p| p.columns.iter().map(|c| c.duration).collect::<Vec<_>>())
                .max_by_key(|c| c.len())
                .filter(|p| p.len() > 1);

            if let Some(col_shape) = col_shape {
                for line in solfa.iter_mut() {
                    self.expand_pulse_at(line, pulse_id, &col_shape);
                }
            }
        }
    }

    fn expand_pulse_at(&self, line: &mut SolfaLineIr, pulse_id: usize, col_shape: &[u8]) {
        let Some(pulse) = line.pulses.get_mut(pulse_id) else {
            return;
        };

        let is_single_empty = matches!(
            pulse.columns.as_slice(),
            [PulseColumn {
                note: NoteKind::EmptyNote,
                ..
            }]
        );

        if is_single_empty {
            pulse.expanded = true;
            pulse.factor = col_shape.iter().sum();
            pulse.columns.clear();

            for &duration in col_shape {
                pulse.columns.push(PulseColumn {
                    duration,
                    underline: UnderlineMarker::default(),
                    note: NoteKind::EmptyNote,
                });
            }
        }
    }

    fn build_pulse_view(&mut self, solfa: &[SolfaLineIr]) -> Vec<PulseView> {
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

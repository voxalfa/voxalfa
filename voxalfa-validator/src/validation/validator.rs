use crate::{
    ast::{
        body::{Body, Section},
        dynamics::Dynamics,
        header::Header,
        lyrics::LyricToken,
        solfa::{PulseAccent, SolfaLine},
        symbols::{SymbolRef, SymbolTree},
        types::TimeSignature,
    },
    diagnostics::{
        reporter::DiagnosticReporter,
        types::{DiagnosticKind, ReportStage},
    },
    ir::{BodyIR, SectionIR, SubSectionIR},
    output::TimelineMap,
    ts_utils::range::RangeUtil,
    validation::timeline::DynamicsBuffer,
};

#[derive(Debug)]
pub struct ValidatorOutput {
    pub timelines: TimelineMap,
    pub reporter: DiagnosticReporter,
}

#[derive(Debug)]
pub struct Validator<'a> {
    pub tree: &'a SymbolTree,
    pub header: &'a Header,
    pub reporter: DiagnosticReporter,
    pub timelines: TimelineMap,
}

impl<'a> Validator<'a> {
    pub fn new(tree: &'a SymbolTree, header: &'a Header) -> Self {
        Self {
            reporter: DiagnosticReporter::new(ReportStage::Validation),
            timelines: TimelineMap::default(),
            header,
            tree,
        }
    }

    pub fn validate_body(&mut self, body: &Body) {
        for section in &body.sections {
            self.validate_pulses(section);
            self.validate_voices(section);
        }

        self.validate_lyrics_join(&body.sections);
    }

    pub fn validate_body_ir(&mut self, body: &BodyIR) {
        let mut buffer = DynamicsBuffer::default();

        for (section_id, section) in body.sections.iter().enumerate() {
            for sub_section in &section.items {
                self.validate_sub_section_ir(sub_section);
            }

            if section.merge {
                self.validate_section_merge(section_id, &body.sections);
            } else {
                buffer.init_section();
            }

            for (sub_id, sub_section) in section.items.iter().enumerate() {
                let dynamics = self.resolve_root_dynamics(section_id, sub_id, &body.sections);

                if let Some(dynamics) = dynamics {
                    buffer.process(sub_section, dynamics);

                    let next_section = body.sections.get(section_id + 1);

                    if next_section.is_some_and(|s| !s.merge) || next_section.is_none() {
                        self.validate_dynamics_buffer(dynamics, &mut buffer);
                    }
                }
            }
        }
    }

    pub fn finalize(self) -> ValidatorOutput {
        ValidatorOutput {
            timelines: self.timelines,
            reporter: self.reporter,
        }
    }

    fn validate_sub_section_ir(&mut self, sub_section: &SubSectionIR) {
        let range = self.tree.get_scope_range(sub_section.sid);

        if let Some(verses) = &self.header.metadata.verses {
            let value = sub_section.lyrics.len();

            if value != verses.value {
                let context_range = self.tree.get_symbol_range(verses.sid);

                self.reporter.error(
                    range,
                    DiagnosticKind::VerseMismatch(verses.value, value, context_range),
                );
            }
        } else if !sub_section.lyrics.is_empty() {
            let context_range = self.tree.get_scope_range(self.header.sid);

            self.reporter.error(
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

                    self.reporter
                        .error(range, DiagnosticKind::TrailingLyric(context_ranges));
                }
            }
        }
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

                    self.reporter.error(
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

                self.reporter
                    .error(range, DiagnosticKind::UndefinedTimeParameter(context_range));
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

                self.reporter.error(
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

            self.reporter.error(
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
            self.reporter.error(
                range,
                DiagnosticKind::VoiceCountMismatch(expected_len, voices.len(), context_range),
            );
        }

        for (id, voice) in voices.iter().enumerate() {
            let range = self.tree.get_symbol_range(voice.sid);

            if let Some(voices) = &self.header.metadata.voices {
                if let Some(expected_voice) = voices.value.get(id) {
                    if voice.value != *expected_voice {
                        self.reporter.error(
                            range,
                            DiagnosticKind::VoiceMismatch(*expected_voice, voice.value),
                        );
                    }
                } else {
                    let context_range = self.tree.get_symbol_range(voices.sid);

                    self.reporter.error(
                        range,
                        DiagnosticKind::UndefinedVoice(voice.value, context_range),
                    );
                }
            } else {
                let context_range = self.tree.get_scope_range(self.header.sid);

                self.reporter
                    .error(range, DiagnosticKind::UndefinedVoiceMetadata(context_range));
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

                        self.reporter.error(
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

                        self.reporter
                            .error(range, DiagnosticKind::UnusedLyricJoin(context_range.end()));
                    }
                }
            }
        }
    }

    fn validate_section_merge(&mut self, section_id: usize, sections: &[SectionIR]) {
        let current = &sections[section_id];
        let root = sections[..section_id].iter().rev().find(|s| !s.merge);

        if let Some(root) = root {
            let current_dist = root.items.iter().map(|sub| sub.solfa.len());
            let target_dist = current.items.iter().map(|sub| sub.solfa.len());

            if !current_dist.eq(target_dist) {
                let range = self.tree.get_scope_range(current.sid);
                let context_range = self.tree.get_scope_range(root.sid);

                self.reporter
                    .error(range, DiagnosticKind::InvalidSectionMerge(context_range));
            }
        }
    }

    fn validate_dynamics_buffer(&mut self, dynamics: &Dynamics, buffer: &mut DynamicsBuffer) {
        if buffer.is_empty() {
            return;
        }

        for (id, dynamic) in dynamics.value.iter().enumerate() {
            if !buffer.has_processed(id) {
                self.reporter.error(
                    self.tree.get_symbol_range(dynamic.sid),
                    DiagnosticKind::UnmatchedDynamic,
                );
            }
        }

        self.timelines.extend_from_buffer(buffer);
    }

    fn resolve_root_dynamics(
        &self,
        section_id: usize,
        sub_id: usize,
        sections: &'a [SectionIR],
    ) -> Option<&'a Dynamics> {
        sections[..=section_id]
            .iter()
            .rev()
            .find(|s| !s.merge)
            .and_then(|s| s.items.get(sub_id).map(|sub| &sub.dynamics))
    }
}

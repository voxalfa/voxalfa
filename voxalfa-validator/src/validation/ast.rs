use crate::{
    ast::{
        lyrics::LyricToken,
        parser::ParserOutput,
        solfa::{Pulse, PulseAccent},
        symbols::SymbolRef,
    },
    data_types::TimeSignature,
    diagnostics::{
        reporter::DiagnosticReporter,
        types::{DiagnosticKind, ReportStage},
    },
    ts_utils::range::RangeUtil,
};
use std::collections::BTreeMap;

#[derive(Debug)]
pub struct AstValidatorOutput {
    pub reporter: DiagnosticReporter,
}

#[derive(Debug)]
pub struct AstValidator<'a> {
    output: &'a ParserOutput,
    reporter: DiagnosticReporter,
}

impl<'a> AstValidator<'a> {
    pub fn new(output: &'a ParserOutput) -> Self {
        Self {
            output,
            reporter: DiagnosticReporter::new(ReportStage::Validation),
        }
    }

    pub fn validate(mut self) -> AstValidatorOutput {
        self.validate_pulses();
        self.validate_voices();
        self.validate_time_signature();
        self.validate_lyrics_join();

        AstValidatorOutput {
            reporter: self.reporter,
        }
    }

    fn validate_pulses(&mut self) {
        for section in &self.output.body.sections {
            let solfa = section
                .items
                .iter()
                .flat_map(|s| &s.solfa)
                .collect::<Vec<_>>();

            let reference = solfa.iter().max_by_key(|s| s.pulses.len());

            for line in solfa.iter() {
                let Some(reference) = reference else { continue };

                let current_len = line.pulses.len();
                let reference_len = reference.pulses.len();

                if current_len != reference_len {
                    let range = self.output.symbols.get_scope_range(line.sid);
                    let context_range = self.output.symbols.get_scope_range(reference.sid);

                    self.reporter.error(
                        range,
                        DiagnosticKind::PulseCountMismatch(
                            reference_len,
                            current_len,
                            context_range,
                        ),
                    );
                }
            }
        }
    }

    fn validate_voices(&mut self) {
        for section in &self.output.body.sections {
            let range = self.output.symbols.get_scope_range(section.sid);

            if let Some(voices_def) = &self.output.header.params.voices {
                let voices = section
                    .items
                    .iter()
                    .flat_map(|sub| sub.solfa.iter().map(|s| &s.voice))
                    .collect::<Vec<_>>();

                let current_len = voices.len();
                let expected_len = voices_def.value.len();
                let context_range = self.output.symbols.get_symbol_range(voices_def.sid);

                if current_len > 0 && current_len != expected_len {
                    self.reporter.error(
                        range,
                        DiagnosticKind::VoiceCountMismatch(
                            expected_len,
                            voices.len(),
                            context_range,
                        ),
                    );
                }

                for (id, voice) in voices.iter().enumerate() {
                    let range = self.output.symbols.get_symbol_range(voice.sid);

                    if let Some(expected) = voices_def.value.get(id) {
                        if voice.value != expected.value {
                            self.reporter.error(
                                range,
                                DiagnosticKind::VoiceMismatch(expected.value, voice.value),
                            );
                        }
                    } else {
                        let context_range = self.output.symbols.get_symbol_range(voices_def.sid);

                        self.reporter.error(
                            range,
                            DiagnosticKind::UndefinedVoice(voice.value, context_range),
                        );
                    }
                }
            } else {
                let context_range = self.output.symbols.get_scope_range(self.output.header.sid);

                self.reporter.error(
                    range,
                    DiagnosticKind::UndefinedVoiceParameter(context_range),
                );
            }
        }
    }

    fn validate_time_signature(&mut self) {
        let sections = &self.output.body.sections;

        if let Some(time) = &self.output.header.params.time {
            let mut groups = vec![(time, Vec::new())];

            for section in sections {
                if let Some(time) = &section.params.time {
                    groups.push((time, Vec::new()));
                }

                let last_index = groups.len() - 1;
                groups[last_index].1.push(section);
            }

            for (time, sections) in groups {
                let mut voice_lines: BTreeMap<_, Vec<&Pulse>> = BTreeMap::new();

                let mapped_pulses = sections.iter().flat_map(|section| {
                    section
                        .items
                        .iter()
                        .flat_map(|item| &item.solfa)
                        .map(|line| &line.pulses)
                        .enumerate()
                });

                for (voice_id, pulses) in mapped_pulses {
                    voice_lines.entry(voice_id).or_default().extend(pulses);
                }

                for lines in voice_lines.values() {
                    self.validate_linear_voice(lines, time);
                }
            }
        } else {
            let solfa_lines = sections
                .iter()
                .flat_map(|section| &section.items)
                .flat_map(|sub| &sub.solfa);

            for line in solfa_lines {
                let range = self.output.symbols.get_scope_range(line.sid);
                let context_range = self.output.symbols.get_scope_range(self.output.header.sid);

                self.reporter
                    .error(range, DiagnosticKind::UndefinedTimeParameter(context_range));
            }
        }
    }

    fn validate_linear_voice(&mut self, pulses: &[&Pulse], time: &SymbolRef<TimeSignature>) {
        let start_offset = pulses
            .iter()
            .position(|p| p.accent.value == PulseAccent::Strong)
            .unwrap_or_default();

        let top = time.value.top as usize;

        for (pulse_id, pulse) in pulses.iter().enumerate() {
            let position = (pulse_id + top - (start_offset % top)) % top;
            let expected = time.value.get_accent(position);

            if pulse.accent.value != expected {
                let range = self.output.symbols.get_symbol_range(pulse.accent.sid);
                let context_range = self.output.symbols.get_symbol_range(time.sid);

                self.reporter.error(
                    range,
                    DiagnosticKind::MismatchedPulseAccent(
                        expected,
                        pulse.accent.value,
                        context_range,
                    ),
                );
            }
        }
    }

    fn validate_lyrics_join(&mut self) {
        let sections = &self.output.body.sections;

        for (id, section) in sections.iter().enumerate() {
            if let Some(next_section) = sections.get(id + 1) {
                if section.params.ending.is_some() {
                    continue;
                }

                for line in section.items.iter().flat_map(|s| &s.lyrics) {
                    if line.anchor.is_none() {
                        let range = self.output.symbols.get_scope_range(line.sid);
                        let context_range = self.output.symbols.get_scope_range(next_section.sid);

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
                        let operator_range = self.output.symbols.get_symbol_range(op.sid);
                        let range = operator_range.merge(anchor_range);
                        let context_range = self.output.symbols.get_scope_range(section.sid);

                        self.reporter
                            .error(range, DiagnosticKind::UnusedLyricJoin(context_range.end()));
                    }
                }
            }
        }
    }
}

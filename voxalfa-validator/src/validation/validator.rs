use std::collections::BTreeMap;

use crate::{
    ast::{
        body::{Body, Section},
        header::Header,
        lyrics::LyricToken,
        params::SubSectionParams,
        solfa::{Pulse, PulseAccent},
        symbols::{SymbolRef, SymbolTree},
    },
    data_types::{TimeSignature, TimedList},
    diagnostics::{
        reporter::DiagnosticReporter,
        types::{DiagnosticKind, ReportStage},
    },
    ir::{BodyIR, SectionIR, SubSectionIR},
    output::event::{TimelineMap, ToEventKind},
    ts_utils::range::RangeUtil,
    validation::event::EventBuffer,
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

        self.validate_time_signature(&body.sections);
        self.validate_lyrics_join(&body.sections);
    }

    pub fn validate_body_ir(&mut self, body: &BodyIR) {
        let mut buffer = EventBuffer::default();

        for (section_id, section) in body.sections.iter().enumerate() {
            for sub_section in &section.items {
                self.validate_sub_section_ir(sub_section);
            }

            if section.merge {
                self.validate_section_merge(section_id, &body.sections);
            } else {
                buffer.init_section();
            }

            buffer.process_section_events(section);

            for (sub_id, sub_section) in section.items.iter().enumerate() {
                let dynamics = self
                    .resolve_root_params(section_id, sub_id, &body.sections)
                    .and_then(|p| p.dynamics.as_ref());

                let touches = section.params.touches.as_ref();
                let next_section = body.sections.get(section_id + 1);
                let is_last = next_section.is_some_and(|s| !s.merge) || next_section.is_none();

                self.process_events(dynamics, sub_section, is_last, &mut buffer);
                self.process_events(touches, sub_section, is_last, &mut buffer);

                if sub_id == section.items.len() - 1 {
                    buffer.add_offset(sub_section.views.len());
                }
            }
        }

        // merge section level events
        self.timelines.extend_from_buffer(&mut buffer);
    }

    pub fn finalize(self) -> ValidatorOutput {
        ValidatorOutput {
            timelines: self.timelines,
            reporter: self.reporter,
        }
    }

    fn process_events<T: ToEventKind>(
        &mut self,
        events: Option<&SymbolRef<TimedList<T>>>,
        sub_section: &SubSectionIR,
        is_last: bool,
        buffer: &mut EventBuffer,
    ) {
        self.validate_timestamps(events);

        if let Some(events) = events {
            buffer.process_local_events(sub_section, &events.value);

            if is_last {
                self.validate_event_buffer(&events.value, buffer);
            }
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

            if current_len != reference.pulses.len() {
                let range = self.tree.get_scope_range(line.sid);
                let context_range = self.tree.get_scope_range(reference.sid);

                self.reporter.error(
                    range,
                    DiagnosticKind::PulseCountMismatch(reference_len, current_len, context_range),
                );
            }
        }
    }

    fn validate_time_signature(&mut self, sections: &[Section]) {
        if let Some(time) = &self.header.params.time {
            let mut groups = vec![(time, Vec::new())];

            for section in sections {
                if let Some(time) = &section.params.time {
                    groups.push((time, Vec::new()));
                }

                let last_index = groups.len() - 1;

                groups[last_index].1.push(section);
            }

            self.validate_time_signature_inner(groups);
        } else {
            let solfa_lines = sections
                .iter()
                .flat_map(|section| &section.items)
                .flat_map(|sub| &sub.solfa);

            for line in solfa_lines {
                let range = self.tree.get_scope_range(line.sid);
                let context_range = self.tree.get_scope_range(self.header.sid);

                self.reporter
                    .error(range, DiagnosticKind::UndefinedTimeParameter(context_range));
            }
        }
    }

    fn validate_time_signature_inner(
        &mut self,
        groups: Vec<(&SymbolRef<TimeSignature>, Vec<&Section>)>,
    ) {
        for (time, sections) in groups {
            let mut voice_lines: BTreeMap<_, Vec<&Pulse>> = BTreeMap::new();

            let solfa_lines = sections
                .iter()
                .flat_map(|section| &section.items)
                .flat_map(|sub| &sub.solfa);

            for line in solfa_lines {
                voice_lines
                    .entry(line.voice.value)
                    .or_default()
                    .extend(&line.pulses);
            }

            for lines in voice_lines.values() {
                self.validate_linear_voice(lines, time);
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
                let range = self.tree.get_symbol_range(pulse.accent.sid);
                let context_range = self.tree.get_symbol_range(time.sid);

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

    fn validate_voices(&mut self, section: &Section) {
        let range = self.tree.get_scope_range(section.sid);

        if let Some(voices_def) = &self.header.params.voices {
            let expected_len = voices_def.value.len();
            let context_range = self.tree.get_symbol_range(voices_def.sid);

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

                if let Some(expected) = voices_def.value.get(id) {
                    if voice.value != expected.value {
                        self.reporter.error(
                            range,
                            DiagnosticKind::VoiceMismatch(expected.value, voice.value),
                        );
                    }
                } else {
                    let context_range = self.tree.get_symbol_range(voices_def.sid);

                    self.reporter.error(
                        range,
                        DiagnosticKind::UndefinedVoice(voice.value, context_range),
                    );
                }
            }
        } else {
            let context_range = self.tree.get_scope_range(self.header.sid);

            self.reporter.error(
                range,
                DiagnosticKind::UndefinedVoiceParameter(context_range),
            );
        };
    }

    fn validate_lyrics_join(&mut self, sections: &[Section]) {
        for (id, section) in sections.iter().enumerate() {
            if let Some(next_section) = sections.get(id + 1) {
                if section.params.ending.is_some() {
                    continue;
                }

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

    fn validate_event_buffer<T: ToEventKind>(
        &mut self,
        events: &TimedList<T>,
        buffer: &mut EventBuffer,
    ) {
        for (id, symbol) in events.iter().enumerate() {
            let event = &symbol.value;
            let kind = event.value.to_event_kind();

            if !buffer.has_processed(kind, id) {
                self.reporter.error(
                    self.tree.get_symbol_range(symbol.sid),
                    DiagnosticKind::UnmatchedTimestamp,
                );
            }
        }

        self.timelines.extend_from_buffer(buffer);
    }

    fn validate_timestamps<T: ToEventKind>(&mut self, events: Option<&SymbolRef<TimedList<T>>>) {
        let Some(events) = events else { return };

        for symbol in &events.value {
            let event = &symbol.value;

            if event.value.is_range() && event.end.is_none() {
                let sid = self.tree.get_symbol_range(symbol.sid);

                self.reporter
                    .error(sid, DiagnosticKind::ExpectedTimestampRange);
            }

            if !event.value.is_range() && event.end.is_some() {
                let sid = self.tree.get_symbol_range(symbol.sid);

                self.reporter.error(sid, DiagnosticKind::RangeNotAllowed);
            }
        }
    }

    fn resolve_root_params(
        &self,
        section_id: usize,
        sub_id: usize,
        sections: &'a [SectionIR],
    ) -> Option<&'a SubSectionParams> {
        sections[..=section_id]
            .iter()
            .rev()
            .find(|s| !s.merge)
            .and_then(|s| s.items.get(sub_id).map(|sub| &sub.params))
    }
}

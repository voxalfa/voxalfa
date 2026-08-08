use crate::{
    ast::{
        header::Header,
        params::SubSectionParams,
        symbols::{SymbolRef, SymbolTree},
    },
    data_types::TimedList,
    diagnostics::{
        reporter::DiagnosticReporter,
        types::{DiagnosticKind, ReportStage},
    },
    ir::{BodyIr, SectionIr, SubSectionIr},
    output::event::{TimelineMap, ToEventKind},
    validation::event::EventBuffer,
};

#[derive(Debug)]
pub struct IrValidatorOutput {
    pub timelines: TimelineMap,
    pub reporter: DiagnosticReporter,
}

#[derive(Debug)]
pub struct IrValidator<'a> {
    header: &'a Header,
    tree: &'a SymbolTree,
    reporter: DiagnosticReporter,
    timelines: TimelineMap,
}

impl<'a> IrValidator<'a> {
    pub fn new(header: &'a Header, tree: &'a SymbolTree) -> Self {
        Self {
            header,
            tree,
            reporter: DiagnosticReporter::new(ReportStage::IRValidation),
            timelines: TimelineMap::default(),
        }
    }

    pub fn validate(mut self, body: &BodyIr) -> IrValidatorOutput {
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

        self.timelines.extend_from_buffer(&mut buffer);

        IrValidatorOutput {
            timelines: self.timelines,
            reporter: self.reporter,
        }
    }

    fn process_events<T: ToEventKind>(
        &mut self,
        events: Option<&SymbolRef<TimedList<T>>>,
        sub_section: &SubSectionIr,
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

    fn validate_sub_section_ir(&mut self, sub_section: &SubSectionIr) {
        let range = self.tree.get_scope_range(sub_section.sid);

        if let Some(verses) = &self.header.metadata.verses {
            let value = sub_section.lyrics.len();

            if !sub_section.solfa.is_empty() && value != verses.value {
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

    fn validate_section_merge(&mut self, section_id: usize, sections: &[SectionIr]) {
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

    fn resolve_root_params<'b>(
        &self,
        section_id: usize,
        sub_id: usize,
        sections: &'b [SectionIr],
    ) -> Option<&'b SubSectionParams> {
        sections[..=section_id]
            .iter()
            .rev()
            .find(|s| !s.merge)
            .and_then(|s| s.items.get(sub_id).map(|sub| &sub.params))
    }
}

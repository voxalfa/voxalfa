use voxalfa_core::{
    ast::{lyrics::LyricOperatorKind, solfa::PulseAccent},
    data_types::{
        Barline, Dynamic, ExtendedTempo, Jump, Key, KeyAccidental, Mark, StaticTempo, TimeSignature,
    },
    ir::{
        PulseView, SectionIr, SubSectionIr,
        lyrics::{LyricColumnIR, LyricLineIr},
    },
    output::{
        FinalOutput,
        lyrics::{LyricsBuilder, LyricsMap},
    },
    validation::event::FLOAT_ERROR,
};

use crate::{
    error::{Error, Result},
    fonts::Fonts,
    locale::Locale,
    model::*,
    options::RenderOptions,
    visitor::SvgVisitor,
};

type PulseId = usize;

pub fn resolve<'a>(
    data: &'a FinalOutput,
    fonts: &Fonts,
    options: &RenderOptions,
    content_width: f32,
) -> Result<ResolvedScore<'a>> {
    let header = build_header(data);

    let default_time = data
        .header
        .get_params(|p| &p.time)
        .copied()
        .ok_or(Error::MissingHeader("time"))?;

    let language = data.header.get_metadata(|m| &m.language);
    let locale = Locale::from_language(language.map(String::as_str));
    let max_factor = data.resolve_maximum_factor();

    let lyrics_builder = LyricsBuilder::new(&fonts.text.lyric);
    let (ref_col, lyrics_map) = lyrics_builder.build_map::<SvgVisitor>(data, max_factor);
    let lead_width = compute_lead_width(fonts);

    let pulse_resolver = PulseResolver::new(data, &locale, lyrics_map, default_time);
    let resolver_data = pulse_resolver.resolve();

    let max_columns = resolver_data
        .pulses
        .iter()
        .map(|pulse| pulse.max_column)
        .max()
        .unwrap_or(1) as f32;

    // let hyphen_width = fonts.text.lyric.width("-");
    // let exact_gap = (ref_col.width - ref_col.exact_width).max(0.0);
    //
    // let lyrics_extra = if exact_gap < hyphen_width {
    //     hyphen_width - exact_gap
    // } else {
    //     0.0
    // };

    let lyrics_width = max_factor as f32 * ref_col.width
        + max_columns * (lead_width + options.elements.lyrics.x_space());
    let note_spacing = options.elements.note.x_space();
    let notes_width =
        max_columns * (options.min_col_width + options.col_padding + lead_width + note_spacing);

    let grid_pulse_width = lyrics_width.max(notes_width);
    let systems = build_systems(resolver_data, grid_pulse_width, content_width);

    Ok(ResolvedScore {
        header,
        systems,
        lead_width,
    })
}

struct PulseResolver<'a, 'd> {
    data: &'a FinalOutput,
    locale: &'d Locale,
    lyrics_map: LyricsMap<f32>,
    pulses: Vec<PulseStack<'a>>,
    spans: Vec<FlatSpan>,
    pending_markers: Vec<(usize, FlatMarker)>,
    current_time: TimeSignature,
    verses: usize,
}

struct SectionContext<'a> {
    section: &'a SectionIr,
    start: usize,
    pulse_count: usize,
    time: TimeSignature,
    time_changed: bool,
    volta: Option<FlatVolta>,
    lyrics: Vec<Vec<LyricLine>>,
    voices: Vec<Vec<VoiceInstance>>,
}

impl<'a, 'b> PulseResolver<'a, 'b> {
    fn new(
        data: &'a FinalOutput,
        locale: &'b Locale,
        lyrics_map: LyricsMap<f32>,
        default_time: TimeSignature,
    ) -> Self {
        let verses = data
            .header
            .get_metadata(|m| &m.verses)
            .copied()
            .unwrap_or_default();

        Self {
            data,
            verses,
            locale,
            lyrics_map,
            pulses: Vec::new(),
            spans: Vec::new(),
            pending_markers: Vec::new(),
            current_time: default_time,
        }
    }

    fn resolve(mut self) -> PulseResolverData<'a> {
        for (section_id, section) in self.data.body.sections.iter().enumerate() {
            self.resolve_section(section_id, section);
        }

        for (pulse_id, marker) in self.pending_markers.drain(..) {
            if let Some(pulse) = self.pulses.get_mut(pulse_id) {
                pulse.markers.push(marker);
            }
        }

        self.compute_measures();

        PulseResolverData {
            pulses: self.pulses,
            spans: self.spans,
            verses: self.verses,
        }
    }

    fn resolve_section(&mut self, section_id: usize, section: &'a SectionIr) {
        let declared_time = section.params.time.as_ref().map(|time| time.value);
        let time_changed = declared_time.is_some_and(|time| time != self.current_time);

        let volta = build_volta(section);
        let lyrics = self.build_section_lyrics(section);
        let voices = self.get_section_voices(section);

        if let Some(time) = declared_time {
            self.current_time = time;
        }

        let pulse_count = section
            .items
            .first()
            .map(|sub| sub.views.len())
            .unwrap_or(0);

        if pulse_count == 0 {
            return;
        }

        let context = SectionContext {
            start: self.pulses.len(),
            time: self.current_time,
            volta,
            lyrics,
            voices,
            section,
            time_changed,
            pulse_count,
        };

        for pulse_id in 0..pulse_count {
            let pulse = build_pulse(&context, pulse_id);
            self.pulses.push(pulse);
        }

        self.add_start_markers(&context);
        self.add_dynamic_markers(section_id, &context);
        self.add_tempo_spans(&context);
        self.add_touch_markers(context.start, section);
        self.add_jump_markers(&context);
        self.add_barline_markers(&context);
        self.mark_section_end(section_id, &context);
    }

    fn get_section_voices(&self, section: &SectionIr) -> Vec<Vec<VoiceInstance>> {
        let mut voices = Vec::with_capacity(section.items.len());
        let mut id = 0;

        for sub in &section.items {
            let mut subsection = Vec::with_capacity(sub.solfa.len());

            for line in &sub.solfa {
                subsection.push(VoiceInstance {
                    id,
                    role: line.voice,
                });

                id += 1;
            }

            voices.push(subsection);
        }

        voices
    }

    fn build_section_lyrics(&self, section: &SectionIr) -> Vec<Vec<LyricLine>> {
        section
            .items
            .iter()
            .map(|sub| {
                if let Some(line) = sub.lyrics.first().filter(|l| l.wildcard) {
                    let line = self.build_lyrics_line(&sub.views, line);

                    vec![line; self.verses]
                } else {
                    sub.lyrics
                        .iter()
                        .map(|line| self.build_lyrics_line(&sub.views, line))
                        .collect()
                }
            })
            .collect()
    }

    fn build_lyrics_line(&self, views: &[PulseView], line: &LyricLineIr) -> LyricLine {
        let mut tokens = Vec::new();

        if views.is_empty() {
            return LyricLine { tokens };
        }

        let mut pulse = 0;
        let mut slot = 0;

        for (lyric_id, column) in line.columns.iter().enumerate() {
            if pulse >= views.len() {
                break;
            }

            let start_pulse = pulse;
            let start_unit = slot_unit(&views[pulse], slot);

            let mut remaining = column.span;

            while remaining > 0 && pulse < views.len() {
                let slot_count = views[pulse].durations.len();

                if slot >= slot_count {
                    pulse += 1;
                    slot = 0;
                    continue;
                }

                let available = slot_count - slot;
                let step = remaining.min(available);

                remaining -= step;
                slot += step;

                if slot >= slot_count {
                    pulse += 1;
                    slot = 0;
                }
            }

            let (end_pulse, end_unit) = if pulse >= views.len() {
                (views.len() - 1, 1.0)
            } else {
                (pulse, slot_unit(&views[pulse], slot))
            };

            let mut text = self
                .lyrics_map
                .get(&column.sid)
                .map(|rendered| rendered.content.clone())
                .unwrap_or_default();

            if let Some(ch) = text.chars().last() {
                let has_concat = line
                    .operators
                    .get(lyric_id)
                    .is_some_and(|operator| operator.value == LyricOperatorKind::Concat);

                if ch.is_alphabetic() && has_concat {
                    text.push('-');
                }
            }

            tokens.push(LyricToken {
                start_pulse,
                start_unit,
                end_pulse,
                end_unit,
                text,
                underline: column_has_underline(column),
            });
        }

        LyricLine { tokens }
    }

    fn compute_measures(&mut self) {
        let mut start = 0;

        while start < self.pulses.len() {
            let time = self.pulses[start].time;
            let mut end = start;

            while end < self.pulses.len() && self.pulses[end].time == time {
                end += 1;
            }

            let top = time.top as usize;

            let offset = (start..end)
                .find(|&index| self.pulses[index].accent == PulseAccent::Strong)
                .map(|index| index - start)
                .unwrap_or(0);

            for index in start..end {
                let relative = index - start;

                self.pulses[index].measure_start =
                    relative >= offset && (relative - offset) % top == 0;
                self.pulses[index].pulse_offset = offset;
            }

            start = end;
        }
    }

    fn add_start_markers(&mut self, context: &SectionContext) {
        let section = context.section;
        let start = context.start;

        if context.time_changed
            && let Some(time) = section.params.time.as_ref().map(|time| time.value)
        {
            self.pulses[start].markers.push(FlatMarker {
                kind: MarkerKind::TimeSignature,
                glyph: MarkerGlyph::Text(format!("{}/{}", time.top, time.bottom)),
                index: 0.0,
                voice: None,
            });
        }

        if let Some(keys) = &section.params.key {
            let positions = section_positions(context);

            for key in &keys.value {
                let key = &key.value;
                let index = if key.implicit { 0.0 } else { key.start };
                let position = resolve_section_position(&positions, context, index, false);

                if let Some((pulse, index)) = position {
                    self.pulses[pulse].markers.push(FlatMarker {
                        kind: MarkerKind::Key,
                        glyph: MarkerGlyph::Text(self.locale.key_label(&key_text(key.value))),
                        index,
                        voice: None,
                    });
                }
            }
        }

        if let Some(tempos) = &section.params.tempo {
            let positions = section_positions(context);

            for tempo in &tempos.value {
                let timed = &tempo.value;
                let tempo = &timed.value;
                let index = if timed.implicit { 0.0 } else { timed.start };
                let position = resolve_section_position(&positions, context, index, false);

                if let Some((pulse, index)) = position {
                    let (kind, label) = match tempo {
                        ExtendedTempo::Progressive(_) => continue,
                        ExtendedTempo::Static(StaticTempo::Custom(t)) => {
                            (MarkerKind::Bpm, format!("M. {t}."))
                        }
                        ExtendedTempo::Static(_) => (MarkerKind::Tempo, format!("{tempo}.")),
                    };

                    self.pulses[pulse].markers.push(FlatMarker {
                        kind,
                        glyph: MarkerGlyph::Text(label),
                        index,
                        voice: None,
                    });
                }
            }
        }
    }

    fn add_dynamic_markers(&mut self, section_id: usize, context: &SectionContext) {
        if context.section.merge {
            return;
        }

        for (sub_id, sub) in context.section.items.iter().enumerate() {
            let views = combined_views(&self.data.body.sections, section_id, sub_id);
            let voice = context.voices[sub_id].first().copied();

            self.add_dynamic_markers_impl(context.start, sub, &views, voice);
        }
    }

    fn add_dynamic_markers_impl(
        &mut self,
        offset: usize,
        sub: &SubSectionIr,
        views: &[PulseView],
        voice: Option<VoiceInstance>,
    ) {
        let Some(dynamics) = &sub.params.dynamics else {
            return;
        };

        let positions = make_indexed_notes(views);

        for timed in &dynamics.value {
            let event = &timed.value;
            let dynamic = event.value;

            let span_kind = match dynamic {
                Dynamic::Cresc => Some(SpanKind::Cresc),
                Dynamic::Decresc => Some(SpanKind::Decresc),
                Dynamic::Dim => Some(SpanKind::Dim),
                _ => None,
            };

            if let Some(kind) = span_kind {
                let Some(start) = resolve_event(&positions, event.start, false) else {
                    continue;
                };

                let Some(end) = event.end.and_then(|e| resolve_event(&positions, e, false)) else {
                    continue;
                };

                self.spans.push(FlatSpan {
                    kind,
                    voice,
                    start_pulse: offset + start.pulse,
                    start_unit: start.unit,
                    end_pulse: offset + end.pulse,
                    end_unit: end.unit,
                });
            } else if let Some(resolved) = resolve_event(&positions, event.start, false) {
                let marker = FlatMarker {
                    kind: MarkerKind::Dynamic,
                    glyph: MarkerGlyph::Text(dynamic.to_string()),
                    index: resolved.unit,
                    voice,
                };

                self.pending_markers.push((offset + resolved.pulse, marker));
            };
        }
    }

    fn add_tempo_spans(&mut self, context: &SectionContext) {
        let Some(tempos) = &context.section.params.tempo else {
            return;
        };

        let positions = section_positions(context);
        let last_index = positions.last().map(|note| note.abs_index).unwrap_or(0.0);

        for tempo in &tempos.value {
            let timed = &tempo.value;

            let ExtendedTempo::Progressive(kind) = timed.value else {
                continue;
            };

            let start_index = if timed.implicit { 0.0 } else { timed.start };

            let end_index = if timed.implicit {
                last_index
            } else {
                timed.end.unwrap_or_default()
            };

            let Some(start) = resolve_event(&positions, start_index, false) else {
                continue;
            };
            let Some(end) = resolve_event(&positions, end_index, false) else {
                continue;
            };

            self.spans.push(FlatSpan {
                kind: SpanKind::Progressive(kind),
                voice: None,
                start_pulse: context.start + start.pulse,
                start_unit: start.unit,
                end_pulse: context.start + end.pulse,
                end_unit: end.unit,
            });
        }
    }

    fn add_jump_markers(&mut self, context: &SectionContext) {
        let section = context.section;
        let positions = section_positions(context);
        let last_index = positions.last().map(|note| note.abs_index).unwrap_or(0.0);

        if let Some(jumps) = &section.params.jump {
            for jump in &jumps.value {
                let jump = &jump.value;

                let index = if jump.implicit {
                    last_index
                } else {
                    jump.start
                };

                let position = resolve_section_position(&positions, context, index, true);

                if let Some((pulse, index)) = position {
                    self.pulses[pulse].markers.push(FlatMarker {
                        kind: MarkerKind::Jump,
                        glyph: MarkerGlyph::Text(jump_label(jump.value, section.get_repeat())),
                        index,
                        voice: None,
                    });
                }
            }
        }

        if let Some(marks) = &section.params.mark {
            for mark in &marks.value {
                let mark = &mark.value;
                let after = matches!(mark.value, Mark::ToCoda | Mark::Fine);
                let glyph = match mark.value {
                    Mark::Segno => MarkerGlyph::Segno,
                    Mark::Coda => MarkerGlyph::Text("Coda".to_string()),
                    Mark::ToCoda => MarkerGlyph::Text("To Coda".to_string()),
                    Mark::Fine => MarkerGlyph::Text("Fine".to_string()),
                };

                let index = if mark.implicit {
                    if after { last_index } else { 0.0 }
                } else {
                    mark.start
                };

                let position = resolve_section_position(&positions, context, index, after);

                if let Some((pulse, unit)) = position {
                    self.pulses[pulse].markers.push(FlatMarker {
                        kind: MarkerKind::Mark,
                        glyph,
                        index: unit,
                        voice: None,
                    });
                }
            }
        }
    }

    fn add_barline_markers(&mut self, context: &SectionContext) {
        let Some(barlines) = &context.section.params.barline else {
            return;
        };

        let positions = section_positions(context);

        for barline in &barlines.value {
            let timed = &barline.value;
            let index = if timed.implicit { 0.0 } else { timed.start };
            let position = resolve_section_position(&positions, context, index, false);

            if let Some((pulse, index)) = position {
                match timed.value {
                    Barline::Double if let Some(pulse) = self.pulses.get_mut(pulse) => {
                        pulse.section_break = Some(index);
                    }
                    _ => {}
                }
            };
        }
    }

    fn mark_section_end(&mut self, section_id: usize, context: &SectionContext) {
        let is_last = self.data.body.sections.len() == section_id + 1;
        let last = context.start + context.pulse_count - 1;

        if is_last && let Some(pulse) = self.pulses.get_mut(last) {
            pulse.section_break = Some(1.0);
        }
    }

    fn add_touch_markers(&mut self, section_start: usize, section: &SectionIr) {
        let touches = section.params.touches.as_ref();
        let views = section.items.first().map(|sub| &sub.views);

        let Some((touches, views)) = touches.zip(views) else {
            return;
        };

        let positions = make_indexed_notes(views);

        for timed in &touches.value {
            let event = &timed.value;

            let Some(resolved) = resolve_event(&positions, event.start, false) else {
                continue;
            };

            if let Some(pulse) = self.pulses.get_mut(section_start + resolved.pulse) {
                pulse.markers.push(FlatMarker {
                    kind: MarkerKind::Touch,
                    glyph: MarkerGlyph::Touch(event.value),
                    index: resolved.unit,
                    voice: None,
                });
            }
        }
    }
}

fn build_volta(section: &SectionIr) -> Option<FlatVolta> {
    section.params.ending.as_ref().map(|ending| FlatVolta {
        number: ending.value,
        label: section
            .params
            .label
            .as_ref()
            .map(|label| label.value.clone()),
    })
}

fn build_pulse<'a>(context: &SectionContext<'a>, pulse_id: usize) -> PulseStack<'a> {
    let mut stack = Vec::new();
    let mut accent = PulseAccent::Strong;
    let mut max_column = 1;

    for (sub_id, sub) in context.section.items.iter().enumerate() {
        let voice_columns = sub
            .solfa
            .iter()
            .filter_map(|line| line.pulses.get(pulse_id))
            .map(|pulse| pulse.columns.len())
            .max()
            .unwrap_or(0);

        max_column = max_column.max(voice_columns);

        let voices = sub
            .solfa
            .iter()
            .zip(&context.voices[sub_id])
            .filter_map(|(line, voice)| line.pulses.get(pulse_id).map(|pulse| (*voice, pulse)))
            .collect::<Vec<_>>();

        if let Some((_, first)) = voices.first() {
            accent = first.accent;
        }

        let lyrics = context
            .lyrics
            .get(sub_id)
            .map(|verses| build_pulse_lyrics(verses, pulse_id, context.start))
            .unwrap_or_default();

        stack.push(PulseStackItem {
            lyric_voices: context.voices[sub_id].clone(),
            lyrics,
            voices,
        });
    }

    PulseStack {
        time: context.time,
        measure_start: false,
        section_break: None,
        pulse_offset: 0,
        markers: Vec::new(),
        volta: context.volta.clone(),
        max_column,
        accent,
        stack,
    }
}

fn build_pulse_lyrics(
    verses: &[LyricLine],
    pulse_id: usize,
    section_start: usize,
) -> Vec<ShiftedLyrics> {
    verses
        .iter()
        .map(|verse| {
            let tokens = verse
                .tokens
                .iter()
                .filter(|token| token.start_pulse == pulse_id)
                .map(|token| token.apply_offset(section_start))
                .collect();

            ShiftedLyrics { tokens }
        })
        .collect()
}

#[derive(Clone, Copy)]
struct IndexedNote {
    pulse: usize,
    rel_index: f32,
    abs_index: f32,
    duration: f32,
}

#[derive(Clone, Copy)]
struct ResolvedEvent {
    pulse: usize,
    unit: f32,
}

fn make_indexed_notes(views: &[PulseView]) -> Vec<IndexedNote> {
    let mut positions = Vec::new();
    let mut abs_offset = 0.0;

    for (pulse_id, view) in views.iter().enumerate() {
        let mut rel_offset = 0.0;

        for &duration in &view.durations {
            let duration = duration as f32 / view.factor as f32;

            positions.push(IndexedNote {
                pulse: pulse_id,
                abs_index: abs_offset,
                rel_index: rel_offset,
                duration,
            });

            rel_offset += duration;
            abs_offset += duration;
        }
    }

    positions
}

fn section_positions(context: &SectionContext) -> Vec<IndexedNote> {
    context
        .section
        .items
        .first()
        .map(|sub| make_indexed_notes(&sub.views))
        .unwrap_or_default()
}

fn resolve_section_position(
    positions: &[IndexedNote],
    context: &SectionContext,
    target: f32,
    after: bool,
) -> Option<(usize, f32)> {
    let resolved = resolve_event(positions, target, after)?;

    Some((context.start + resolved.pulse, resolved.unit))
}

fn resolve_event(positions: &[IndexedNote], target: f32, after: bool) -> Option<ResolvedEvent> {
    positions.iter().find_map(|note| {
        ((note.abs_index - target).abs() < FLOAT_ERROR).then_some(ResolvedEvent {
            pulse: note.pulse,
            unit: note.rel_index + if after { note.duration } else { 0.0 },
        })
    })
}

fn combined_views(sections: &[SectionIr], section_id: usize, sub_id: usize) -> Vec<PulseView> {
    let mut views = Vec::new();

    if let Some(sub) = sections[section_id].items.get(sub_id) {
        views.extend(sub.views.iter().cloned());
    }

    for section in &sections[section_id + 1..] {
        if !section.merge {
            break;
        }

        if let Some(sub) = section.items.get(sub_id) {
            views.extend(sub.views.iter().cloned());
        }
    }

    views
}

fn key_text(key: Key) -> String {
    let mut text = key.to_string();

    match key.accidental {
        KeyAccidental::Neutral => {}
        KeyAccidental::Flat => {
            text.pop();
            text.push('♭');
        }
        KeyAccidental::Sharp => {
            text.pop();
            text.push('♯');
        }
    }

    text
}

fn jump_label(jump: Jump, repeat: Option<usize>) -> String {
    let label = match jump {
        Jump::DS => "D.S.",
        Jump::DC => "D.C.",
        Jump::DSC => "D.S. al Coda",
        Jump::DSF => "D.S. al Fine",
        Jump::DCC => "D.C. al Coda",
        Jump::DCF => "D.C. al Fine",
    };

    match repeat {
        Some(count) => format!("{label} x {count}"),
        None => label.to_string(),
    }
}

fn build_header(data: &FinalOutput) -> HeaderView {
    let title = data.header.get_metadata(|m| &m.title).cloned();
    let description = data.header.get_metadata(|m| &m.description).cloned();
    let language = data.header.get_metadata(|m| &m.language).cloned();

    let authors = data
        .header
        .get_metadata(|m| &m.author)
        .map(|author| {
            author
                .iter()
                .map(|item| item.value.clone())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let composers = data
        .header
        .get_metadata(|m| &m.composer)
        .map(|composer| {
            composer
                .iter()
                .map(|item| item.value.clone())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let key = data.header.get_params(|p| &p.key).map(|key| key_text(*key));

    let time = data
        .header
        .get_params(|p| &p.time)
        .map(|time| format!("{}/{}", time.top, time.bottom));

    let tempo_param = data.header.get_params(|p| &p.tempo);
    let is_bpm = tempo_param.is_some_and(|tempo| matches!(tempo, StaticTempo::Custom(_)));

    let tempo = tempo_param.map(|tempo| {
        if matches!(tempo, StaticTempo::Custom(_)) {
            format!("M. {tempo}.")
        } else {
            format!("{tempo}.")
        }
    });

    let meter = data.header.get_metadata(|m| &m.meter).map(|meter| {
        meter
            .iter()
            .map(|item| item.value.to_string())
            .collect::<Vec<_>>()
            .join(".")
    });

    HeaderView {
        title,
        authors,
        composers,
        language,
        description,
        key,
        time,
        tempo,
        is_bpm,
        meter,
        rows: data.header.directives.rows.clone(),
    }
}

fn column_has_underline(column: &LyricColumnIR) -> bool {
    column
        .chunks
        .iter()
        .flat_map(|chunk| &chunk.primitives)
        .any(|primitive| primitive.underline.left || primitive.underline.right)
}

fn slot_unit(view: &PulseView, slot: usize) -> f32 {
    let total: u32 = view.durations.iter().map(|&duration| duration as u32).sum();

    let elapsed: u32 = view
        .durations
        .iter()
        .take(slot)
        .map(|&duration| duration as u32)
        .sum();

    elapsed as f32 / total as f32
}

fn compute_lead_width(fonts: &Fonts) -> f32 {
    fonts
        .text
        .note
        .width(".,")
        .max(fonts.text.note.width("!"))
        .max(fonts.text.note.width(":"))
        .max(4.0)
}

fn build_systems<'a>(
    resolver_data: PulseResolverData<'a>,
    pulse_width: f32,
    max_width: f32,
) -> Vec<System<'a>> {
    let mut measures: Vec<Measure> = Vec::new();
    let mut start = 0;

    for (index, pulse) in resolver_data.pulses.iter().enumerate() {
        if pulse.measure_start || pulse.time != resolver_data.pulses[start].time {
            measures.push(Measure { start, end: index });
            start = index;
        }
    }

    if start < resolver_data.pulses.len() {
        measures.push(Measure {
            start,
            end: resolver_data.pulses.len(),
        });
    }

    let mut systems = Vec::new();
    let mut sys_measures = Vec::new();
    let mut width = 0.0;
    let mut time: Option<TimeSignature> = None;
    let mut first_system = true;
    let mut pickup_offset = 0;

    for measure in measures {
        let config = &resolver_data.pulses[measure.start];
        let measure_time = config.time;
        let pulse_offset = config.pulse_offset;

        let measure_width = (measure.end - measure.start) as f32 * pulse_width;
        let time_changed = time.is_some_and(|current| current != measure_time);

        let overflow_limit = if first_system {
            max_width
        } else {
            (max_width - pickup_offset as f32 * pulse_width).max(0.0)
        };

        let overflow = !sys_measures.is_empty() && width + measure_width > overflow_limit;
        let offset = if first_system { 0 } else { pickup_offset };

        if time_changed {
            if !sys_measures.is_empty() {
                systems.push(make_system(
                    &resolver_data,
                    std::mem::take(&mut sys_measures),
                    pulse_width,
                    offset,
                ));
            }

            first_system = true;
            pickup_offset = pulse_offset;
            width = 0.0;
        } else if overflow {
            systems.push(make_system(
                &resolver_data,
                std::mem::take(&mut sys_measures),
                pulse_width,
                offset,
            ));

            first_system = false;
            width = 0.0;
        } else if time.is_none() {
            pickup_offset = pulse_offset;
        }

        time = Some(measure_time);
        sys_measures.push(measure);
        width += measure_width;
    }

    if !sys_measures.is_empty() {
        let offset = if first_system { 0 } else { pickup_offset };

        systems.push(make_system(
            &resolver_data,
            sys_measures,
            pulse_width,
            offset,
        ));
    }

    systems
}

fn make_system<'a>(
    resolver_data: &PulseResolverData<'a>,
    measures: Vec<Measure>,
    pulse_width: f32,
    grid_offset: usize,
) -> System<'a> {
    let time = resolver_data.pulses[measures[0].start].time;
    let layout = SystemLayout::new(&resolver_data.pulses, measures, resolver_data.verses);
    let (voices, lyric_groups) = layout.lyrics();
    let pulses = layout.system_pulses(pulse_width);
    let mut pulse_offsets = Vec::with_capacity(pulses.len());
    let mut x = 0.0;

    for pulse in &pulses {
        pulse_offsets.push(x);
        x += pulse.pulse_width;
    }

    System {
        pulses,
        pulse_offsets,
        markers: layout.markers(),
        spans: layout.spans(&resolver_data.spans),
        voltas: layout.voltas(),
        pulse_offset: grid_offset,
        lyric_groups,
        voices,
        time,
    }
}

struct SystemLayout<'a, 'b> {
    pulses: &'b [PulseStack<'a>],
    measures: Vec<Measure>,
    verses: usize,
    length: usize,
    start: usize,
    end: usize,
}

impl<'a, 'b> SystemLayout<'a, 'b> {
    fn new(pulses: &'b [PulseStack<'a>], measures: Vec<Measure>, verses: usize) -> Self {
        let start = measures[0].start;
        let end = measures.last().map(|l| l.end).unwrap_or(start);
        let length = end - start;

        Self {
            measures,
            pulses,
            verses,
            length,
            end,
            start,
        }
    }

    fn len(&self) -> usize {
        self.length
    }

    fn pulses(&self) -> impl Iterator<Item = PulseId> {
        self.measures.iter().flat_map(|m| m.start..m.end)
    }

    fn spans(&self, spans: &[FlatSpan]) -> Vec<SystemSpan> {
        spans
            .iter()
            .filter(|span| span.end_pulse >= self.start && span.start_pulse < self.end)
            .map(|span| {
                let ends_after = span.end_pulse >= self.end;

                SystemSpan {
                    kind: span.kind,
                    voice: span.voice,
                    start: span.start_pulse.saturating_sub(self.start),
                    start_unit: span.start_unit,
                    end: if ends_after {
                        self.len() - 1
                    } else {
                        (span.end_pulse - self.start).min(self.len() - 1)
                    },
                    end_unit: if ends_after { 1.0 } else { span.end_unit },
                    fits: span.start_pulse >= self.start && span.end_pulse < self.end,
                    continued: span.start_pulse < self.start,
                }
            })
            .collect()
    }

    fn system_pulses(&self, pulse_width: f32) -> Vec<SystemPulse<'a>> {
        self.pulses()
            .map(|index| {
                let pulse = &self.pulses[index];

                let stack = pulse
                    .stack
                    .iter()
                    .map(|sub| SystemPulseStackItem {
                        voices: sub.voices.clone(),
                    })
                    .collect();

                SystemPulse {
                    accent: pulse.accent,
                    measure_start: pulse.measure_start,
                    section_break: pulse.section_break,
                    pulse_width,
                    stack,
                }
            })
            .collect()
    }

    fn markers(&self) -> Vec<SystemMarker> {
        self.pulses()
            .enumerate()
            .flat_map(|(pulse, global)| {
                self.pulses[global]
                    .markers
                    .iter()
                    .map(move |marker| SystemMarker {
                        pulse,
                        unit: marker.index,
                        kind: marker.kind,
                        glyph: marker.glyph.clone(),
                        voice: marker.voice,
                    })
            })
            .collect()
    }

    fn voltas(&self) -> Vec<SystemVolta> {
        let mut voltas = Vec::new();
        let mut current: Option<(usize, FlatVolta)> = None;

        let first = self
            .pulses
            .get(self.start)
            .and_then(|pulse| pulse.volta.as_ref());

        let previous = self
            .start
            .checked_sub(1)
            .and_then(|pulse| self.pulses.get(pulse))
            .and_then(|pulse| pulse.volta.as_ref());

        let continues_previous = first.is_some() && first == previous;

        for (local, global) in self.pulses().enumerate() {
            let Some(volta) = &self.pulses[global].volta else {
                if let Some((start, volta)) = current.take() {
                    voltas.push(SystemVolta::new(
                        start,
                        local,
                        volta,
                        continues_previous && start == 0,
                    ));
                }

                continue;
            };

            let continues = current.as_ref().is_some_and(|(_, active)| active == volta);

            if continues {
                continue;
            }

            if let Some((start, active)) = current.take() {
                voltas.push(SystemVolta::new(
                    start,
                    local,
                    active,
                    continues_previous && start == 0,
                ));
            }

            current = Some((local, volta.clone()));
        }

        if let Some((start, volta)) = current {
            voltas.push(SystemVolta::new(
                start,
                self.len(),
                volta,
                continues_previous && start == 0,
            ));
        }

        if let Some(last) = voltas.last_mut() {
            let continues = self
                .pulses
                .get(self.end)
                .and_then(|pulse| pulse.volta.as_ref())
                .is_some_and(|volta| volta.number == last.number && volta.label == last.label);

            let last_of_piece = self.pulses[self.end..]
                .iter()
                .all(|pulse| pulse.volta.is_none());

            last.open_end = continues || last_of_piece;
        }

        voltas
    }

    // Two section with different voice set might share the same line
    // so we must merge lyrics for visual consistency
    fn lyrics(&self) -> (Vec<VoiceInstance>, Vec<SystemLyricGroup>) {
        let voices = self.resolve_voice_set();
        let voice_lyrics = self.collect_lyrics(&voices);
        let groups = merge_lyric_groups(&voices, &voice_lyrics);

        (voices, groups)
    }

    fn resolve_voice_set(&self) -> Vec<VoiceInstance> {
        let mut voices: Vec<VoiceInstance> = Vec::new();

        for pulse_id in self.pulses() {
            for sub in &self.pulses[pulse_id].stack {
                for &voice in &sub.lyric_voices {
                    if !voices.iter().any(|item| item.id == voice.id) {
                        voices.push(voice);
                    }
                }
            }
        }

        voices
    }

    /// Collects the lyrics of every voice separately. A lyric line declared by
    /// a sub-section belongs to every voice of that sub-section, so a shared
    /// line is copied onto each voice. Voices that end up with identical lines
    /// are merged again when the rows are emitted.
    fn collect_lyrics(&self, voices: &[VoiceInstance]) -> Vec<SystemVoiceLyrics> {
        let mut voice_lyrics = vec![SystemVoiceLyrics::new(self.verses); voices.len()];

        for (rel_id, abs_id) in self.pulses().enumerate() {
            for sub in &self.pulses[abs_id].stack {
                let lyrics = sub.lyrics.iter().enumerate().flat_map(|(verse, line)| {
                    line.tokens
                        .iter()
                        .filter(|token| token.start_pulse == abs_id && !token.text.is_empty())
                        .map(move |token| (verse, token))
                });

                for (verse, token) in lyrics {
                    let system_token = self.make_system_lyric_token(rel_id, token);

                    for &voice in &sub.lyric_voices {
                        if let Some(index) = voices.iter().position(|item| item.id == voice.id) {
                            voice_lyrics[index].verses[verse].push(system_token.clone());
                        }
                    }
                }
            }
        }

        voice_lyrics
    }

    fn make_system_lyric_token(&self, rel_id: usize, token: &LyricToken) -> SystemLyricToken {
        let end = token
            .end_pulse
            .saturating_sub(self.start)
            .min(self.len() - 1);

        let end_unit = if token.end_pulse >= self.end {
            1.0
        } else {
            token.end_unit
        };

        SystemLyricToken {
            start: rel_id,
            start_unit: token.start_unit,
            end,
            end_unit,
            text: token.text.clone(),
            underline: token.underline,
        }
    }
}

/// Merges consecutive voices whose collected lyrics are identical. The
/// resulting group is emitted once, after the last voice it contains.
fn merge_lyric_groups(
    voices: &[VoiceInstance],
    voice_lyrics: &[SystemVoiceLyrics],
) -> Vec<SystemLyricGroup> {
    let mut groups = Vec::new();
    let mut start = 0;

    while start < voices.len() {
        let mut end = start + 1;

        while end < voices.len() && voice_lyrics[start] == voice_lyrics[end] {
            end += 1;
        }

        let lyrics = &voice_lyrics[start];

        if !lyrics.is_empty() {
            groups.push(SystemLyricGroup {
                voices: voices[start..end].to_vec(),
                verses: lyrics.verses.clone(),
            });
        }

        start = end;
    }

    groups
}

use voxalfa_core::{
    ast::solfa::PulseAccent,
    data_types::{
        Dynamic, ExtendedTempo, Jump, Key, KeyAccidental, Mark, StaticTempo, TimeSignature, Voice,
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
    let (pulses, dynamic_spans) = pulse_resolver.resolve();

    let max_columns = pulses
        .iter()
        .map(|pulse| pulse.max_column)
        .max()
        .unwrap_or(1) as f32;

    let lyrics_width = max_factor as f32 * ref_col.width + max_columns * lead_width;
    let notes_width = max_columns * (options.min_col_width + options.col_padding + lead_width);

    let grid_pulse_width = lyrics_width.max(notes_width);
    let systems = build_systems(&pulses, &dynamic_spans, grid_pulse_width, content_width);

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
    dynamic_spans: Vec<FlatDynamicSpan>,
    current_time: TimeSignature,
}

struct SectionContext<'a> {
    section: &'a SectionIr,
    start: usize,
    pulse_count: usize,
    time: TimeSignature,
    time_changed: bool,
    volta: Option<FlatVolta>,
    lyrics: Vec<Vec<LyricLine>>,
}

impl<'a, 'b> PulseResolver<'a, 'b> {
    fn new(
        data: &'a FinalOutput,
        locale: &'b Locale,
        lyrics_map: LyricsMap<f32>,
        default_time: TimeSignature,
    ) -> Self {
        Self {
            data,
            locale,
            lyrics_map,
            pulses: Vec::new(),
            dynamic_spans: Vec::new(),
            current_time: default_time,
        }
    }

    fn resolve(mut self) -> (Vec<PulseStack<'a>>, Vec<FlatDynamicSpan>) {
        for (section_id, section) in self.data.body.sections.iter().enumerate() {
            self.resolve_section(section_id, section);
        }

        self.compute_measures();

        (self.pulses, self.dynamic_spans)
    }

    fn resolve_section(&mut self, section_id: usize, section: &'a SectionIr) {
        let declared_time = section.params.time.as_ref().map(|time| time.value);
        let time_changed = declared_time.is_some_and(|time| time != self.current_time);

        let volta = build_volta(section);
        let lyrics = build_section_lyrics(section, &mut self.lyrics_map);

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
        self.add_touch_markers(context.start, section);
        self.add_jump_markers(&context);
        self.mark_section_end(section_id, &context);
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
        let pulse = &mut self.pulses[context.start];

        if context.time_changed
            && let Some(time) = section.params.time.as_ref().map(|time| time.value)
        {
            pulse.markers.push(FlatMarker {
                kind: MarkerKind::TimeSignature,
                glyph: MarkerGlyph::Text(format!("{}/{}", time.top, time.bottom)),
                index: 0.0,
                voice: None,
            });
        }

        if let Some(key) = section.params.key.as_ref().map(|key| key.value) {
            pulse.markers.push(FlatMarker {
                kind: MarkerKind::Key,
                glyph: MarkerGlyph::Text(self.locale.key_label(&key_text(key))),
                index: 0.0,
                voice: None,
            });
        }

        if let Some(tempo) = section.params.tempo.as_ref().map(|tempo| tempo.value) {
            let (kind, label) = match tempo {
                ExtendedTempo::Static(StaticTempo::Custom(t)) => {
                    (MarkerKind::Bpm, format!("M {t}"))
                }
                _ => (MarkerKind::Tempo, tempo.to_string()),
            };

            pulse.markers.push(FlatMarker {
                kind,
                glyph: MarkerGlyph::Text(label),
                index: 0.0,
                voice: None,
            });
        }
    }

    fn add_dynamic_markers(&mut self, section_id: usize, context: &SectionContext) {
        if context.section.merge {
            return;
        }

        for (sub_id, sub) in context.section.items.iter().enumerate() {
            let views = combined_views(&self.data.body.sections, section_id, sub_id);

            self.add_dynamic_markers_impl(context.start, sub, &views);
        }
    }

    fn add_dynamic_markers_impl(
        &mut self,
        section_start: usize,
        sub: &SubSectionIr,
        views: &[PulseView],
    ) {
        let Some(dynamics) = &sub.params.dynamics else {
            return;
        };

        let positions = make_indexed_notes(views);

        let voice = match sub.solfa.as_slice() {
            [line] => Some(line.voice),
            _ => None,
        };

        for timed in &dynamics.value {
            let event = &timed.value;
            let dynamic = event.value;

            if matches!(dynamic, Dynamic::Cre | Dynamic::Dec) {
                let Some(end) = event.end else {
                    continue;
                };

                let Some((start_pulse, start_unit)) = resolve_event(&positions, event.start) else {
                    continue;
                };

                let Some((end_pulse, end_unit)) = resolve_event(&positions, end) else {
                    continue;
                };

                self.dynamic_spans.push(FlatDynamicSpan {
                    kind: match dynamic {
                        Dynamic::Cre => DynamicSpanKind::Crescendo,
                        Dynamic::Dec => DynamicSpanKind::Decrescendo,
                        _ => unreachable!(),
                    },
                    start_pulse: section_start + start_pulse,
                    start_unit,
                    end_pulse: section_start + end_pulse,
                    end_unit,
                });

                continue;
            }

            let Some((pulse_id, unit)) = resolve_event(&positions, event.start) else {
                continue;
            };

            let Some(pulse) = self.pulses.get_mut(section_start + pulse_id) else {
                continue;
            };

            pulse.markers.push(FlatMarker {
                kind: MarkerKind::Dynamic,
                glyph: MarkerGlyph::Text(dynamic.to_string()),
                index: unit,
                voice,
            });
        }
    }

    fn add_jump_markers(&mut self, context: &SectionContext) {
        let section = context.section;
        let last = context.start + context.pulse_count - 1;

        if let Some(jump) = &section.params.jump {
            self.pulses[last].markers.push(FlatMarker {
                kind: MarkerKind::Jump,
                glyph: MarkerGlyph::Text(jump_label(jump.value, section.get_repeat())),
                index: 1.0,
                voice: None,
            });
        }

        if let Some(mark) = &section.params.mark {
            let (glyph, pulse, unit) = match mark.value {
                Mark::Segno => (MarkerGlyph::Segno, context.start, 0.0),
                Mark::Coda => (MarkerGlyph::Text("Coda".to_string()), context.start, 0.0),
                Mark::ToCoda => (MarkerGlyph::Text("To Coda".to_string()), last, 1.0),
                Mark::Fine => (MarkerGlyph::Text("Fine".to_string()), last, 1.0),
            };

            self.pulses[pulse].markers.push(FlatMarker {
                kind: MarkerKind::Mark,
                glyph,
                index: unit,
                voice: None,
            });
        }
    }

    fn mark_section_end(&mut self, section_id: usize, context: &SectionContext) {
        let remaining = &self.data.body.sections[section_id + 1..];
        let is_last = remaining.is_empty();
        let next_is_major = remaining.first().is_some_and(|section| section.major_start);
        let last = context.start + context.pulse_count - 1;

        if (next_is_major || is_last)
            && let Some(pulse) = self.pulses.get_mut(last)
        {
            pulse.section_end = true;
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

            let Some((pulse_id, index)) = resolve_event(&positions, event.start) else {
                continue;
            };

            if let Some(pulse) = self.pulses.get_mut(section_start + pulse_id) {
                pulse.markers.push(FlatMarker {
                    kind: MarkerKind::Touch,
                    glyph: MarkerGlyph::Touch(event.value),
                    index,
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

fn build_section_lyrics(
    section: &SectionIr,
    lyrics_map: &mut LyricsMap<f32>,
) -> Vec<Vec<LyricLine>> {
    section
        .items
        .iter()
        .map(|sub| {
            sub.lyrics
                .iter()
                .map(|line| build_lyrics_line(&sub.views, line, lyrics_map))
                .collect()
        })
        .collect()
}

fn build_pulse<'a>(context: &SectionContext<'a>, pulse_id: usize) -> PulseStack<'a> {
    let mut stack = Vec::new();
    let mut accent = PulseAccent::Strong;
    let mut max_column = 1;

    for (sub_id, sub) in context.section.items.iter().enumerate() {
        let Some(view) = sub.views.get(pulse_id) else {
            continue;
        };

        max_column = max_column.max(view.durations.len());

        let voices = sub
            .solfa
            .iter()
            .filter_map(|line| line.pulses.get(pulse_id).map(|pulse| (line.voice, pulse)))
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
            lyric_voices: sub.solfa.iter().map(|s| s.voice).collect(),
            lyrics,
            voices,
        });
    }

    PulseStack {
        time: context.time,
        measure_start: false,
        section_end: false,
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

fn make_indexed_notes(views: &[PulseView]) -> Vec<IndexedNote> {
    let mut positions = Vec::new();
    let mut abs_offset = 0.0;

    for (pulse_id, view) in views.iter().enumerate() {
        let total = view
            .durations
            .iter()
            .map(|&duration| duration as u32)
            .sum::<u32>();

        let mut rel_offset = 0;

        for &duration in &view.durations {
            let duration = duration as f32 / view.factor as f32;

            positions.push(IndexedNote {
                pulse: pulse_id,
                abs_index: abs_offset,
                rel_index: rel_offset as f32 / total as f32,
                duration: duration / view.factor as f32,
            });

            rel_offset += duration as u32;
            abs_offset += duration;
        }
    }

    positions
}

fn resolve_event(positions: &[IndexedNote], target: f32) -> Option<(usize, f32)> {
    positions.iter().find_map(|note| {
        if (note.abs_index - target).abs() < FLOAT_ERROR {
            Some((note.pulse, note.rel_index))
        } else if (note.abs_index + note.duration - target).abs() < FLOAT_ERROR {
            Some((note.pulse, note.rel_index + note.duration))
        } else {
            None
        }
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
            format!("M {tempo}")
        } else {
            tempo.to_string()
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
    }
}

fn build_lyrics_line(
    views: &[PulseView],
    line: &LyricLineIr,
    lyrics_map: &mut LyricsMap<f32>,
) -> LyricLine {
    let mut tokens = Vec::new();

    if views.is_empty() {
        return LyricLine { tokens };
    }

    let mut pulse = 0;
    let mut slot = 0;

    for column in &line.columns {
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

        let text = lyrics_map
            .remove(&column.sid)
            .map(|rendered| rendered.content.clone())
            .unwrap_or_default();

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
    pulses: &[PulseStack<'a>],
    dynamic_spans: &[FlatDynamicSpan],
    pulse_width: f32,
    max_width: f32,
) -> Vec<System<'a>> {
    let mut measures: Vec<Measure> = Vec::new();
    let mut start = 0;

    for (index, pulse) in pulses.iter().enumerate() {
        if pulse.measure_start || pulse.time != pulses[start].time {
            measures.push(Measure { start, end: index });
            start = index;
        }
    }

    if start < pulses.len() {
        measures.push(Measure {
            start,
            end: pulses.len(),
        });
    }

    let mut systems = Vec::new();
    let mut sys_measures = Vec::new();
    let mut width = 0.0;
    let mut time: Option<TimeSignature> = None;
    let mut first_system = true;
    let mut pickup_offset = 0;

    for measure in measures {
        let measure_time = pulses[measure.start].time;
        let pulse_offset = pulses[measure.start].pulse_offset;

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
                    pulses,
                    dynamic_spans,
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
                pulses,
                dynamic_spans,
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
            pulses,
            dynamic_spans,
            sys_measures,
            pulse_width,
            offset,
        ));
    }

    systems
}

fn make_system<'a>(
    pulses: &[PulseStack<'a>],
    spans: &[FlatDynamicSpan],
    measures: Vec<Measure>,
    pulse_width: f32,
    grid_offset: usize,
) -> System<'a> {
    let time = pulses[measures[0].start].time;
    let layout = SystemLayout::new(pulses, measures);
    let (voices, lyric_groups) = layout.lyrics();

    System {
        pulses: layout.system_pulses(pulse_width),
        lyric_groups,
        markers: layout.markers(),
        dynamic_spans: layout.dynamic_spans(spans),
        voltas: layout.voltas(),
        pulse_offset: grid_offset,
        voices,
        time,
    }
}

struct SystemLayout<'a, 'b> {
    pulses: &'b [PulseStack<'a>],
    measures: Vec<Measure>,
    length: usize,
    start: usize,
    end: usize,
}

impl<'a, 'b> SystemLayout<'a, 'b> {
    fn new(pulses: &'b [PulseStack<'a>], measures: Vec<Measure>) -> Self {
        let start = measures[0].start;
        let end = measures.last().map(|l| l.end).unwrap_or(start);
        let length = end - start;

        Self {
            measures,
            pulses,
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

    fn dynamic_spans(&self, spans: &[FlatDynamicSpan]) -> Vec<SystemDynamicSpan> {
        spans
            .iter()
            .filter(|span| span.end_pulse >= self.start && span.start_pulse < self.end)
            .map(|span| SystemDynamicSpan {
                kind: span.kind,
                start: span.start_pulse.saturating_sub(self.start),
                start_unit: span.start_unit,
                end: (span.end_pulse - self.start).min(self.len() - 1),
                end_unit: span.end_unit,
                fits: span.start_pulse >= self.start && span.end_pulse < self.end,
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
                    section_end: pulse.section_end,
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

        for (local, global) in self.pulses().enumerate() {
            let Some(volta) = &self.pulses[global].volta else {
                if let Some((start, volta)) = current.take() {
                    voltas.push(SystemVolta::new(start, local, volta));
                }

                continue;
            };

            let continues = current.as_ref().is_some_and(|(_, active)| active == volta);

            if continues {
                continue;
            }

            if let Some((start, active)) = current.take() {
                voltas.push(SystemVolta::new(start, local, active));
            }

            current = Some((local, volta.clone()));
        }

        if let Some((start, volta)) = current {
            voltas.push(SystemVolta::new(start, self.len(), volta));
        }

        voltas
    }

    // Two section with different voice set might share the same line
    // so we must merge lyrics for visual consistency
    fn lyrics(&self) -> (Vec<Voice>, Vec<SystemLyricGroup>) {
        let (voices, max_verses) = self.resolve_lyrics_params();
        let voice_lyrics = self.collect_lyrics(&voices, max_verses);
        let groups = merge_lyric_groups(&voices, &voice_lyrics);

        (voices, groups)
    }

    fn resolve_lyrics_params(&self) -> (Vec<Voice>, usize) {
        let mut voices = Vec::new();
        let mut max_verses = 0;

        for pulse_id in self.pulses() {
            for sub in &self.pulses[pulse_id].stack {
                max_verses = max_verses.max(sub.lyrics.len());

                for &voice in &sub.lyric_voices {
                    if !voices.contains(&voice) {
                        voices.push(voice);
                    }
                }
            }
        }

        (voices, max_verses)
    }

    /// Collects the lyrics of every voice separately. A lyric line declared by
    /// a sub-section belongs to every voice of that sub-section, so a shared
    /// line is copied onto each voice. Voices that end up with identical lines
    /// are merged again when the rows are emitted.
    fn collect_lyrics(&self, voices: &[Voice], max_verses: usize) -> Vec<SystemVoiceLyrics> {
        let mut voice_lyrics = vec![SystemVoiceLyrics::new(max_verses); voices.len()];

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
                        if let Some(index) = voices.iter().position(|item| *item == voice) {
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
    voices: &[Voice],
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

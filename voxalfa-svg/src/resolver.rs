use voxalfa_core::{
    ast::solfa::PulseAccent,
    data_types::{
        Dynamic, ExtendedTempo, Jump, Key, KeyAccidental, Mark, StaticTempo, TimeSignature, Touch,
        Voice,
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
    model::{
        DynamicSpanKind, FlatDynamicSpan, FlatMarker, FlatPulse, FlatSubPulse, FlatVolta,
        HeaderView, LyricToken, MarkerGlyph, MarkerKind, ResolvedScore, System, SystemDynamicSpan,
        SystemLyricGroup, SystemLyricToken, SystemMarker, SystemPulse, SystemSubPulse, SystemVolta,
    },
    options::RenderOptions,
    visitor::SvgVisitor,
};

type PulseId = usize;

pub fn resolve<'a>(
    data: &'a FinalOutput,
    fonts: &Fonts,
    options: &RenderOptions,
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

    let lyrics_builder = LyricsBuilder::new(&fonts.lyric);
    let (lyric_width, lyrics_map) = lyrics_builder.build_map::<SvgVisitor>(data, max_factor);
    let (col_width, lead_width) = compute_widths(fonts, options, lyric_width);

    let pulse_resolver = PulseResolver::new(data, &locale, lyrics_map, default_time);
    let (pulses, dynamic_spans) = pulse_resolver.resolve();

    let max_columns = pulses
        .iter()
        .map(|pulse| pulse.max_columns)
        .max()
        .unwrap_or(1);

    let grid_pulse_width = max_columns.max(1) as f32 * (col_width + lead_width);

    let systems = build_systems(
        &pulses,
        &dynamic_spans,
        grid_pulse_width,
        options.printable_width(),
    );

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
    pulses: Vec<FlatPulse<'a>>,
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
    lyrics: Vec<Vec<Vec<LyricToken>>>,
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

    fn resolve(mut self) -> (Vec<FlatPulse<'a>>, Vec<FlatDynamicSpan>) {
        for (section_id, section) in self.data.body.sections.iter().enumerate() {
            self.resolve_section(section_id, section);
        }

        self.compute_measures();

        (self.pulses, self.dynamic_spans)
    }

    fn resolve_section(&mut self, section_id: usize, section: &'a SectionIr) {
        let declared_time = section.params.time.as_ref().map(|time| time.value);
        let time_changed = declared_time.is_some_and(|time| time != self.current_time);

        if let Some(time) = declared_time {
            self.current_time = time;
        }

        let pulse_count = section
            .items
            .iter()
            .map(|sub| sub.views.len())
            .max()
            .unwrap_or(0);

        if pulse_count == 0 {
            return;
        }

        let context = SectionContext {
            start: self.pulses.len(),
            time: self.current_time,
            volta: build_volta(section),
            lyrics: build_section_lyrics(section, &mut self.lyrics_map),
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
                unit: 0.0,
            });
        }

        if let Some(key) = section.params.key.as_ref().map(|key| key.value) {
            pulse.markers.push(FlatMarker {
                kind: MarkerKind::Key,
                glyph: MarkerGlyph::Text(self.locale.key_label(&key_text(key))),
                unit: 0.0,
            });
        }

        if let Some(tempo) = section.params.tempo.as_ref().map(|tempo| tempo.value) {
            let kind = match tempo {
                ExtendedTempo::Static(StaticTempo::Custom(_)) => MarkerKind::TempoNumber,
                _ => MarkerKind::Tempo,
            };

            pulse.markers.push(FlatMarker {
                kind,
                glyph: MarkerGlyph::Text(tempo.to_string()),
                unit: 0.0,
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

        let positions = note_positions(views);

        for timed in &dynamics.value {
            let event = &timed.value;
            let dynamic = event.value;

            if matches!(dynamic, Dynamic::Cre | Dynamic::Dec) {
                let Some(end) = event.end else {
                    continue;
                };

                let Some((start_pulse, start_unit)) = event_unit(&positions, event.start) else {
                    continue;
                };

                let Some((end_pulse, end_unit)) = event_unit(&positions, end) else {
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

            let Some((pulse_id, unit)) = event_unit(&positions, event.start) else {
                continue;
            };

            let text = dynamic.to_string();

            let Some(pulse) = self.pulses.get_mut(section_start + pulse_id) else {
                continue;
            };

            pulse.markers.push(FlatMarker {
                kind: MarkerKind::Dynamic,
                glyph: MarkerGlyph::Text(text),
                unit,
            });
        }
    }

    fn add_jump_markers(&mut self, context: &SectionContext) {
        let section = context.section;
        let last = context.start + context.pulse_count - 1;

        if let Some(jump) = &section.params.jump {
            self.pulses[last].markers.push(FlatMarker {
                kind: MarkerKind::Jump,
                glyph: MarkerGlyph::Text(jump_label(
                    jump.value,
                    section
                        .params
                        .repeat
                        .as_ref()
                        .map(|repeat| repeat.value as u8),
                )),
                unit: 1.0,
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
                unit,
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
        let Some(touches) = &section.params.touches else {
            return;
        };
        let Some(sub) = section.items.first() else {
            return;
        };

        let positions = note_positions(&sub.views);

        for timed in &touches.value {
            let event = &timed.value;
            let Some((pulse_id, unit)) = event_unit(&positions, event.start) else {
                continue;
            };

            let glyph = match event.value {
                Touch::Fermata => MarkerGlyph::Fermata,
                Touch::Staccato => MarkerGlyph::Staccato,
                Touch::Marcato => MarkerGlyph::Marcato,
            };

            if let Some(pulse) = self.pulses.get_mut(section_start + pulse_id) {
                pulse.markers.push(FlatMarker {
                    kind: MarkerKind::Touch,
                    glyph,
                    unit,
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
) -> Vec<Vec<Vec<LyricToken>>> {
    section
        .items
        .iter()
        .map(|sub| {
            sub.lyrics
                .iter()
                .map(|line| build_lyric_tokens(&sub.views, line, lyrics_map))
                .collect()
        })
        .collect()
}

fn build_pulse<'a>(context: &SectionContext<'a>, pulse_id: usize) -> FlatPulse<'a> {
    let mut subsections = Vec::new();
    let mut max_columns = 1;
    let mut accent = PulseAccent::Strong;

    for (sub_id, sub) in context.section.items.iter().enumerate() {
        let Some(view) = sub.views.get(pulse_id) else {
            continue;
        };

        max_columns = max_columns.max(view.durations.len());

        let voices = sub
            .solfa
            .iter()
            .filter_map(|line| line.pulses.get(pulse_id).map(|pulse| (line.voice, pulse)))
            .collect::<Vec<_>>();

        if let Some((_, first)) = voices.first() {
            accent = first.accent;
        }

        for (_, pulse) in &voices {
            max_columns = max_columns.max(pulse.columns.len().max(1));
        }

        let mut lyric_voices = Vec::new();

        for line in &sub.solfa {
            if !lyric_voices.contains(&line.voice) {
                lyric_voices.push(line.voice);
            }
        }

        let lyrics = context
            .lyrics
            .get(sub_id)
            .map(|verses| build_pulse_lyrics(verses, pulse_id, context.start))
            .unwrap_or_default();

        subsections.push(FlatSubPulse {
            voices,
            lyric_voices,
            lyrics,
        });
    }

    FlatPulse {
        time: context.time,
        accent,
        measure_start: false,
        section_end: false,
        pulse_offset: 0,
        subsections,
        markers: Vec::new(),
        volta: context.volta.clone(),
        max_columns,
    }
}

fn build_pulse_lyrics(
    verses: &[Vec<LyricToken>],
    pulse_id: usize,
    section_start: usize,
) -> Vec<Vec<LyricToken>> {
    verses
        .iter()
        .map(|verse| {
            verse
                .iter()
                .filter(|token| token.start_pulse == pulse_id)
                .map(|token| {
                    let mut token = token.clone();
                    token.start_pulse += section_start;
                    token.end_pulse += section_start;
                    token
                })
                .collect()
        })
        .collect()
}

#[derive(Clone, Copy)]
struct NotePosition {
    pulse: usize,
    start: f32,
    end: f32,
    unit_start: f32,
    unit_end: f32,
}

fn note_positions(views: &[PulseView]) -> Vec<NotePosition> {
    let mut positions = Vec::new();
    let mut relative = 0.0;

    for (pulse_id, view) in views.iter().enumerate() {
        let total = view
            .durations
            .iter()
            .map(|&duration| duration as u32)
            .sum::<u32>()
            .max(1);

        let mut cursor = 0;

        for &duration in &view.durations {
            let start = relative;
            let end = relative + duration as f32 / view.factor as f32;
            let unit_start = cursor as f32 / total as f32;

            cursor += duration as u32;

            positions.push(NotePosition {
                pulse: pulse_id,
                start,
                end,
                unit_start,
                unit_end: cursor as f32 / total as f32,
            });

            relative = end;
        }
    }

    positions
}

fn event_unit(positions: &[NotePosition], target: f32) -> Option<(usize, f32)> {
    let position = positions
        .iter()
        .find(|position| (position.start - target).abs() < FLOAT_ERROR)
        .or_else(|| {
            positions
                .iter()
                .find(|position| (position.end - target).abs() < FLOAT_ERROR)
        })?;

    let unit = if (position.start - target).abs() < FLOAT_ERROR {
        position.unit_start
    } else {
        position.unit_end
    };

    Some((position.pulse, unit))
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

    if key.accidental == KeyAccidental::Flat {
        text.pop();
        text.push('♭');
    }

    text
}

fn jump_label(jump: Jump, repeat: Option<u8>) -> String {
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

    let tempo = data
        .header
        .get_params(|p| &p.tempo)
        .map(|tempo| tempo.to_string());

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
        meter,
    }
}

fn build_lyric_tokens(
    views: &[PulseView],
    line: &LyricLineIr,
    lyrics_map: &mut LyricsMap<f32>,
) -> Vec<LyricToken> {
    let mut tokens = Vec::new();

    if views.is_empty() {
        return tokens;
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

    tokens
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

    if total == 0 {
        return 0.0;
    }

    let clamped = slot.min(view.durations.len());

    let elapsed: u32 = view
        .durations
        .iter()
        .take(clamped)
        .map(|&duration| duration as u32)
        .sum();

    elapsed as f32 / total as f32
}

fn compute_widths(fonts: &Fonts, options: &RenderOptions, lyric_width: f32) -> (f32, f32) {
    let lead_width = fonts
        .solfa
        .width(".,")
        .max(fonts.solfa.width("!"))
        .max(fonts.solfa.width(":"))
        .max(4.0);

    let col_width = lyric_width
        .min(options.max_lyric_width)
        .max(options.min_col_width)
        + options.col_padding;

    (col_width, lead_width)
}

fn build_systems<'a>(
    pulses: &[FlatPulse<'a>],
    dynamic_spans: &[FlatDynamicSpan],
    pulse_width: f32,
    max_width: f32,
) -> Vec<System<'a>> {
    let mut measures: Vec<Vec<PulseId>> = Vec::new();
    let mut current: Vec<PulseId> = Vec::new();

    for (index, pulse) in pulses.iter().enumerate() {
        if !current.is_empty() && (pulse.measure_start || pulse.time != pulses[current[0]].time) {
            measures.push(std::mem::take(&mut current));
        }

        current.push(index);
    }

    if !current.is_empty() {
        measures.push(current);
    }

    // Pack whole measures into systems. Every measure has the same pulse
    // width, so its width is simply `pulse_count * pulse_width`.
    let mut systems = Vec::new();
    let mut indices = Vec::new();
    let mut width = 0.0;
    let mut time: Option<TimeSignature> = None;
    let mut first_system_in_group = true;
    let mut pickup_offset = 0;

    for measure in measures {
        let measure_time = pulses[measure[0]].time;
        let measure_width = measure.len() as f32 * pulse_width;
        let time_changed = time.is_some_and(|current| current != measure_time);
        let overflow = !indices.is_empty() && width + measure_width > max_width;

        if time_changed {
            if !indices.is_empty() {
                let offset = if first_system_in_group {
                    0
                } else {
                    pickup_offset
                };

                systems.push(make_system(
                    pulses,
                    dynamic_spans,
                    std::mem::take(&mut indices),
                    pulse_width,
                    offset,
                ));
            }

            first_system_in_group = true;
            pickup_offset = pulses[measure[0]].pulse_offset;
            width = 0.0;
        } else if overflow {
            let offset = if first_system_in_group {
                0
            } else {
                pickup_offset
            };

            systems.push(make_system(
                pulses,
                dynamic_spans,
                std::mem::take(&mut indices),
                pulse_width,
                offset,
            ));

            first_system_in_group = false;
            width = 0.0;
        } else if time.is_none() {
            pickup_offset = pulses[measure[0]].pulse_offset;
        }

        time = Some(measure_time);
        indices.extend(measure);
        width += measure_width;
    }

    if !indices.is_empty() {
        let offset = if first_system_in_group {
            0
        } else {
            pickup_offset
        };

        systems.push(make_system(
            pulses,
            dynamic_spans,
            indices,
            pulse_width,
            offset,
        ));
    }

    systems
}

fn make_system<'a>(
    pulses: &[FlatPulse<'a>],
    spans: &[FlatDynamicSpan],
    indices: Vec<PulseId>,
    pulse_width: f32,
    grid_offset: usize,
) -> System<'a> {
    let layout = SystemLayout::new(pulses, &indices);
    let (voices, lyric_groups) = layout.lyrics();

    System {
        pulses: layout.system_pulses(pulse_width),
        voices,
        lyric_groups,
        markers: layout.markers(),
        dynamic_spans: layout.dynamic_spans(spans),
        voltas: layout.voltas(),
        pulse_offset: grid_offset,
    }
}

struct SystemLayout<'a, 'b> {
    pulses: &'b [FlatPulse<'a>],
    indices: &'b [usize],
    start: usize,
    end: usize,
}

impl<'a, 'p> SystemLayout<'a, 'p> {
    fn new(pulses: &'p [FlatPulse<'a>], indices: &'p [usize]) -> Self {
        let start = indices[0];

        Self {
            pulses,
            indices,
            start,
            end: start + indices.len(),
        }
    }

    fn len(&self) -> usize {
        self.indices.len()
    }

    fn dynamic_spans(&self, spans: &[FlatDynamicSpan]) -> Vec<SystemDynamicSpan> {
        spans
            .iter()
            .filter(|span| span.end_pulse >= self.start && span.start_pulse < self.end)
            .map(|span| {
                let start = span.start_pulse.saturating_sub(self.start);

                let start_unit = if span.start_pulse < self.start {
                    0.0
                } else {
                    span.start_unit
                };

                let end = (span.end_pulse - self.start).min(self.len() - 1);

                let end_unit = if span.end_pulse >= self.end {
                    1.0
                } else {
                    span.end_unit
                };

                let fits = span.start_pulse >= self.start && span.end_pulse < self.end;

                SystemDynamicSpan {
                    kind: span.kind,
                    start,
                    start_unit,
                    end,
                    end_unit,
                    fits,
                }
            })
            .collect()
    }

    fn system_pulses(&self, pulse_width: f32) -> Vec<SystemPulse<'a>> {
        self.indices
            .iter()
            .map(|&index| {
                let pulse = &self.pulses[index];

                SystemPulse {
                    accent: pulse.accent,
                    measure_start: pulse.measure_start,
                    section_end: pulse.section_end,
                    pulse_width,
                    subsections: pulse
                        .subsections
                        .iter()
                        .map(|sub| SystemSubPulse {
                            voices: sub.voices.clone(),
                        })
                        .collect(),
                }
            })
            .collect()
    }

    fn markers(&self) -> Vec<SystemMarker> {
        self.indices
            .iter()
            .enumerate()
            .flat_map(|(pulse, &global)| {
                self.pulses[global]
                    .markers
                    .iter()
                    .map(move |marker| SystemMarker {
                        pulse,
                        unit: marker.unit,
                        kind: marker.kind,
                        glyph: marker.glyph.clone(),
                    })
            })
            .collect()
    }

    fn voltas(&self) -> Vec<SystemVolta> {
        let mut voltas = Vec::new();
        let mut current: Option<(usize, FlatVolta)> = None;

        for (local, &global) in self.indices.iter().enumerate() {
            let Some(volta) = &self.pulses[global].volta else {
                if let Some((start, volta)) = current.take() {
                    voltas.push(volta_span(start, local, volta));
                }

                continue;
            };

            let continues = current.as_ref().is_some_and(|(_, active)| active == volta);

            if continues {
                continue;
            }

            if let Some((start, active)) = current.take() {
                voltas.push(volta_span(start, local, active));
            }

            current = Some((local, volta.clone()));
        }

        if let Some((start, volta)) = current {
            voltas.push(volta_span(start, self.len(), volta));
        }

        voltas
    }

    fn lyrics(&self) -> (Vec<Voice>, Vec<SystemLyricGroup>) {
        let (voices, max_verses) = self.voices();
        let voice_lyrics = self.collect_lyrics(&voices, max_verses);
        let groups = merge_lyric_groups(&voices, &voice_lyrics);

        (voices, groups)
    }

    /// Voice order is the order of first appearance across all sub-sections.
    /// The same voice may be declared by different sub-sections, but it still
    /// owns exactly one row in the system.
    fn voices(&self) -> (Vec<Voice>, usize) {
        let mut voices = Vec::new();
        let mut max_verses = 0;

        for &global in self.indices {
            for sub in &self.pulses[global].subsections {
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
    fn collect_lyrics(
        &self,
        voices: &[Voice],
        max_verses: usize,
    ) -> Vec<Vec<Vec<SystemLyricToken>>> {
        let mut voice_lyrics = vec![vec![Vec::new(); max_verses]; voices.len()];

        for (local, &global) in self.indices.iter().enumerate() {
            let pulse = &self.pulses[global];

            for sub in &pulse.subsections {
                for (verse, tokens) in sub.lyrics.iter().enumerate() {
                    for token in tokens {
                        if token.start_pulse != global || token.text.is_empty() {
                            continue;
                        }

                        let end = token
                            .end_pulse
                            .saturating_sub(self.start)
                            .min(self.len() - 1);

                        let end_unit = if token.end_pulse >= self.end {
                            1.0
                        } else {
                            token.end_unit
                        };

                        let system_token = SystemLyricToken {
                            start: local,
                            start_unit: token.start_unit,
                            end,
                            end_unit,
                            text: token.text.clone(),
                            underline: token.underline,
                        };

                        for &voice in &sub.lyric_voices {
                            if let Some(voice_index) = voices.iter().position(|item| *item == voice)
                            {
                                voice_lyrics[voice_index][verse].push(system_token.clone());
                            }
                        }
                    }
                }
            }
        }

        voice_lyrics
    }
}

fn volta_span(start: usize, end: usize, volta: FlatVolta) -> SystemVolta {
    SystemVolta {
        start,
        end,
        number: volta.number,
        label: volta.label,
    }
}

/// Merges consecutive voices whose collected lyrics are identical. The
/// resulting group is emitted once, after the last voice it contains.
fn merge_lyric_groups(
    voices: &[Voice],
    voice_lyrics: &[Vec<Vec<SystemLyricToken>>],
) -> Vec<SystemLyricGroup> {
    let mut groups = Vec::new();
    let mut start = 0usize;

    while start < voices.len() {
        let mut end = start + 1;

        while end < voices.len() && voice_lyrics[start] == voice_lyrics[end] {
            end += 1;
        }

        if voice_lyrics[start].iter().any(|tokens| !tokens.is_empty()) {
            groups.push(SystemLyricGroup {
                voices: voices[start..end].to_vec(),
                verses: voice_lyrics[start].clone(),
            });
        }

        start = end;
    }

    groups
}

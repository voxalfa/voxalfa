use voxalfa_core::{
    ast::{lyrics::LyricOperatorKind, solfa::PulseAccent},
    data_types::TimeSignature,
    ir::{
        PulseView,
        lyrics::{LyricColumnIR, LyricLineIr, LyricStringIR},
        solfa::NoteKind,
    },
    output::FinalOutput,
};

use crate::{
    error::{Error, Result},
    fonts::Fonts,
    model::{
        FlatPulse, FlatSubPulse, HeaderView, LyricToken, ResolvedScore, System, SystemLyricGroup,
        SystemLyricToken, SystemPulse, SystemSubPulse,
    },
    options::RenderOptions,
};

/// Build the complete, geometry-independent rendering model.
pub(crate) fn resolve<'a>(
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

    let mut pulses = Vec::new();
    let mut current_time = default_time;

    for (section_id, section) in data.body.sections.iter().enumerate() {
        if let Some(time) = &section.params.time {
            current_time = time.value;
        }

        let section_start = pulses.len();
        let pulse_count = section
            .items
            .iter()
            .map(|sub| sub.views.len())
            .max()
            .unwrap_or(0);

        if pulse_count == 0 {
            continue;
        }

        // Resolve lyric columns once per sub-section and verse. The result is
        // stored by the pulse each token starts on, which keeps the later
        // system splitting simple.
        let lyrics: Vec<Vec<Vec<LyricToken>>> = section
            .items
            .iter()
            .map(|sub| {
                sub.lyrics
                    .iter()
                    .map(|line| build_lyric_tokens(data, &sub.views, line))
                    .collect()
            })
            .collect();

        for pulse_id in 0..pulse_count {
            let mut subsections = Vec::new();
            let mut max_columns = 1;
            let mut accent = PulseAccent::Weak;

            for (sub_id, sub) in section.items.iter().enumerate() {
                let Some(view) = sub.views.get(pulse_id) else {
                    continue;
                };

                max_columns = max_columns.max(view.durations.len().max(1));

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

                let mut pulse_lyrics = Vec::new();

                for verse in &lyrics[sub_id] {
                    let mut tokens = Vec::new();

                    for token in verse {
                        if token.start_pulse == pulse_id {
                            let mut token = token.clone();
                            token.start_pulse += section_start;
                            token.end_pulse += section_start;
                            tokens.push(token);
                        }
                    }

                    pulse_lyrics.push(tokens);
                }

                let mut lyric_voices = Vec::new();

                for line in &sub.solfa {
                    if !lyric_voices.contains(&line.voice) {
                        lyric_voices.push(line.voice);
                    }
                }

                subsections.push(FlatSubPulse {
                    voices,
                    lyric_voices,
                    lyrics: pulse_lyrics,
                });
            }

            pulses.push(FlatPulse {
                time: current_time,
                accent,
                measure_start: false,
                section_end: false,
                grid_offset: 0,
                subsections,
                max_columns,
            });
        }

        let next_is_major = data
            .body
            .sections
            .get(section_id + 1)
            .is_some_and(|s| s.major_start);

        if (next_is_major || section_id == data.body.sections.len() - 1)
            && let Some(last) = pulses.get_mut(section_start + pulse_count - 1)
        {
            last.section_end = true;
        }
    }

    compute_measures(&mut pulses);

    let (col_width, lead_width) = compute_widths(&pulses, fonts, options);

    // A single pulse width is used for the whole score, so every pulse and
    // every measure is vertically aligned across systems and pages.
    let max_columns = pulses
        .iter()
        .map(|pulse| pulse.max_columns)
        .max()
        .unwrap_or(1);
    let grid_pulse_width = max_columns.max(1) as f32 * (col_width + lead_width);

    let systems = build_systems(&pulses, grid_pulse_width, options.printable_width());

    Ok(ResolvedScore {
        header,
        systems,
        col_width,
        lead_width,
    })
}

fn build_header(data: &FinalOutput) -> HeaderView {
    let title = data.header.get_metadata(|m| &m.title).cloned();
    let description = data.header.get_metadata(|m| &m.description).cloned();

    let mut authors = Vec::new();

    if let Some(author) = data.header.get_metadata(|m| &m.author) {
        authors.extend(author.iter().map(|item| item.value.clone()));
    }

    if let Some(composer) = data.header.get_metadata(|m| &m.composer) {
        authors.extend(composer.iter().map(|item| item.value.clone()));
    }

    let key = data
        .header
        .get_params(|p| &p.key)
        .map(|key| key.to_string());
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
            .join("+")
    });

    HeaderView {
        title,
        authors,
        description,
        key,
        time,
        tempo,
        meter,
    }
}

/// Convert lyric columns into tokens with `(pulse, unit)` positions.
///
/// `span` is the number of *logical note columns* consumed by a token. The
/// logical column shape comes from `PulseView`, so a token automatically
/// collapses to a whole pulse when the voices in that sub-section disagree.
fn build_lyric_tokens(
    data: &FinalOutput,
    views: &[PulseView],
    line: &LyricLineIr,
) -> Vec<LyricToken> {
    let mut tokens = Vec::new();

    if views.is_empty() {
        return tokens;
    }

    let mut pulse = 0usize;
    let mut slot = 0usize;

    for column in &line.columns {
        if pulse >= views.len() {
            break;
        }

        let start_pulse = pulse;
        let start_unit = slot_unit(&views[pulse], slot);
        let span = column.span.max(1);
        let mut remaining = span;

        while remaining > 0 && pulse < views.len() {
            let len = views[pulse].durations.len();

            if slot >= len {
                pulse += 1;
                slot = 0;
                continue;
            }

            let available = len - slot;
            let take = remaining.min(available);

            remaining -= take;
            slot += take;

            if slot >= len {
                pulse += 1;
                slot = 0;
            }
        }

        let (end_pulse, end_unit) = if pulse >= views.len() {
            (views.len() - 1, 1.0)
        } else {
            (pulse, slot_unit(&views[pulse], slot))
        };

        let (text, placeholder, underline) = render_lyric_column(data, column);

        tokens.push(LyricToken {
            start_pulse,
            start_unit,
            end_pulse,
            end_unit,
            span,
            text,
            placeholder,
            underline,
        });
    }

    tokens
}

fn render_lyric_column(data: &FinalOutput, column: &LyricColumnIR) -> (String, bool, bool) {
    if column.placeholder {
        return (String::new(), true, false);
    }

    let mut text = String::new();
    let mut underline = false;

    for (chunk_id, chunk) in column.chunks.iter().enumerate() {
        for primitive in &chunk.primitives {
            if primitive.underline.left || primitive.underline.right {
                underline = true;
            }

            match &primitive.string {
                LyricStringIR::Reference(id) => {
                    text.push_str(data.symbols.get_lyric_chunk(*id));
                }
                LyricStringIR::Special(ch) => {
                    text.push_str(&ch.to_string());
                }
            }
        }

        if let Some(operator) = column.operators.get(chunk_id) {
            match operator {
                LyricOperatorKind::Space | LyricOperatorKind::Newline => text.push(' '),
                LyricOperatorKind::Concat => {}
            }
        }
    }

    (text, false, underline)
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

/// Measures are resolved from the time signature, not from section
/// boundaries. Pulses from all sections are treated as one continuous line,
/// exactly like the AST validator does.
fn compute_measures(pulses: &mut [FlatPulse]) {
    let mut start = 0usize;

    while start < pulses.len() {
        let time = pulses[start].time;
        let mut end = start;

        while end < pulses.len() && pulses[end].time == time {
            end += 1;
        }

        let top = (time.top as usize).max(1);
        let offset = (start..end)
            .find(|&index| pulses[index].accent == PulseAccent::Strong)
            .map(|index| index - start)
            .unwrap_or(0);

        for index in start..end {
            let relative = index - start;
            pulses[index].measure_start = relative >= offset && (relative - offset) % top == 0;
            pulses[index].grid_offset = offset;
        }

        start = end;
    }
}

fn compute_widths(pulses: &[FlatPulse], fonts: &Fonts, options: &RenderOptions) -> (f32, f32) {
    let mut max_lyric = options.min_col_width;
    let mut max_note = 0.0f32;

    for pulse in pulses {
        for sub in &pulse.subsections {
            for verse in &sub.lyrics {
                for token in verse {
                    // A token that spans several notes spreads its text over
                    // several grid columns, so its full width is not a good
                    // column-width candidate.
                    if !token.text.is_empty() && token.span <= 1 {
                        max_lyric = max_lyric.max(fonts.lyric.width(&token.text));
                    }
                }
            }

            for (_, voice_pulse) in &sub.voices {
                for column in &voice_pulse.columns {
                    let base = match &column.note {
                        NoteKind::Note(note) => fonts.solfa.width(&note.text()),
                        NoteKind::ProlongedNote => fonts.solfa.width("-"),
                        NoteKind::EmptyNote => 0.0,
                    };

                    let width = match &column.note {
                        NoteKind::Note(note) if note.octave != 0 => {
                            base + fonts.octave.width("1") + 1.0
                        }
                        _ => base,
                    };

                    max_note = max_note.max(width);
                }
            }
        }
    }

    let lead_width = fonts
        .solfa
        .width(".,")
        .max(fonts.solfa.width("!"))
        .max(fonts.solfa.width(":"))
        .max(4.0);

    let max_lyric = max_lyric.min(options.max_lyric_width);
    let col_width = max_lyric.max(max_note).max(options.min_col_width) + options.col_padding;

    (col_width, lead_width)
}

fn build_systems<'a>(
    pulses: &[FlatPulse<'a>],
    pulse_width: f32,
    max_width: f32,
) -> Vec<System<'a>> {
    // Split into measures first. The pulse before the first strong accent is
    // an anacrusis and forms its own (partial) measure.
    let mut measures: Vec<Vec<usize>> = Vec::new();
    let mut current: Vec<usize> = Vec::new();

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
    let mut width = 0.0f32;
    let mut time: Option<TimeSignature> = None;
    let mut first_system_in_group = true;
    let mut pickup_offset = 0usize;

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
                    std::mem::take(&mut indices),
                    pulse_width,
                    offset,
                ));
            }

            first_system_in_group = true;
            pickup_offset = pulses[measure[0]].grid_offset;
            width = 0.0;
        } else if overflow {
            let offset = if first_system_in_group {
                0
            } else {
                pickup_offset
            };

            systems.push(make_system(
                pulses,
                std::mem::take(&mut indices),
                pulse_width,
                offset,
            ));

            first_system_in_group = false;
            width = 0.0;
        } else if time.is_none() {
            pickup_offset = pulses[measure[0]].grid_offset;
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

        systems.push(make_system(pulses, indices, pulse_width, offset));
    }

    systems
}

fn make_system<'a>(
    pulses: &[FlatPulse<'a>],
    indices: Vec<usize>,
    pulse_width: f32,
    grid_offset: usize,
) -> System<'a> {
    let system_start = indices[0];
    let system_end = system_start + indices.len();

    let system_pulses = indices
        .iter()
        .map(|&index| {
            let pulse = &pulses[index];

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
        .collect();

    // Voice order is the order of first appearance across all sub-sections.
    // The same voice may be declared by different sub-sections, but it still
    // owns exactly one row in the system.
    let mut voices = Vec::new();
    let mut max_verses = 0usize;

    for &global in &indices {
        for sub in &pulses[global].subsections {
            max_verses = max_verses.max(sub.lyrics.len());

            for &voice in &sub.lyric_voices {
                if !voices.contains(&voice) {
                    voices.push(voice);
                }
            }
        }
    }

    // Collect the lyrics of every voice separately. A lyric line declared by
    // a sub-section belongs to every voice of that sub-section, so a shared
    // line is copied onto each voice. Voices that end up with identical lines
    // are merged again when the rows are emitted.
    let mut voice_lyrics: Vec<Vec<Vec<SystemLyricToken>>> =
        vec![vec![Vec::new(); max_verses]; voices.len()];

    for (local, &global) in indices.iter().enumerate() {
        let pulse = &pulses[global];

        for sub in &pulse.subsections {
            for (verse, tokens) in sub.lyrics.iter().enumerate() {
                for token in tokens {
                    if token.start_pulse != global || token.text.is_empty() {
                        continue;
                    }

                    let end = token
                        .end_pulse
                        .saturating_sub(system_start)
                        .min(indices.len() - 1);
                    let end_unit = if token.end_pulse >= system_end {
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
                        placeholder: token.placeholder,
                        underline: token.underline,
                    };

                    for &voice in &sub.lyric_voices {
                        let Some(voice_index) = voices.iter().position(|item| *item == voice)
                        else {
                            continue;
                        };

                        voice_lyrics[voice_index][verse].push(system_token.clone());
                    }
                }
            }
        }
    }

    // Merge consecutive voices whose collected lyrics are identical. The
    // resulting group is emitted once, after the last voice it contains.
    let mut lyric_groups = Vec::new();
    let mut start = 0usize;

    while start < voices.len() {
        let mut end = start + 1;

        while end < voices.len() && voice_lyrics[start] == voice_lyrics[end] {
            end += 1;
        }

        if voice_lyrics[start].iter().any(|tokens| !tokens.is_empty()) {
            lyric_groups.push(SystemLyricGroup {
                voices: voices[start..end].to_vec(),
                verses: voice_lyrics[start].clone(),
            });
        }

        start = end;
    }

    System {
        pulses: system_pulses,
        voices,
        lyric_groups,
        grid_offset,
    }
}

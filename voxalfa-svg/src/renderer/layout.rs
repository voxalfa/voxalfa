use voxalfa_core::{data_types::Voice, ir::solfa::NoteKind};

use crate::{
    fonts::Fonts,
    layout::*,
    model::{DynamicSpanKind, MarkerGlyph, MarkerKind, System, SystemMarker},
    options::RenderOptions,
};

use super::{
    geometry::{
        find_voice, note_text, prolonged_leading, pulse_offsets, pulse_usable_width, segno_width,
    },
    markers::{marker_font, marker_priority, spread_markers, touch_width},
    primitives::TextAnchor,
};

const MARKER_SAME_NOTE_EPSILON: f32 = 0.001;

pub fn layout_system(
    system: &System,
    fonts: &Fonts,
    lead: f32,
    options: &RenderOptions,
) -> SystemLayout {
    let has_voltas = !system.voltas.is_empty();

    let (mut jumps, jump_height) = if has_voltas {
        layout_jumps(system, fonts, lead)
    } else {
        (Vec::new(), 0.0)
    };

    let (mut voltas, volta_height) = layout_voltas(system, fonts, lead);
    let (mut markers, marker_height) = layout_markers(system, fonts, lead, has_voltas);

    let dynamic_center = markers
        .iter()
        .find(|marker| marker.kind == MarkerKind::Dynamic)
        .map(|marker| marker.y - (fonts.dynamic.ascent() - fonts.dynamic.descent()) / 2.0);

    let (mut dynamic_spans, dynamic_span_height) =
        layout_dynamic_spans(system, fonts, lead, dynamic_center, marker_height);

    // A jump above a volta shares the volta's text row so it sits directly
    // on top of the bracket line instead of floating above the ending label.
    let jump_offset = if voltas.is_empty() { jump_height } else { 0.0 };

    let volta_top = if voltas.is_empty() {
        0.0
    } else {
        VOLTA_TOP_GAP
    };

    for jump in &mut jumps {
        jump.y += volta_top;
    }

    for volta in &mut voltas {
        volta.text_y += volta_top + jump_offset;
        volta.line_y += volta_top + jump_offset;
    }

    let marker_shift = volta_top + jump_offset + volta_height;

    for marker in &mut markers {
        marker.y += marker_shift;
    }

    for span in &mut dynamic_spans {
        span.center_y += marker_shift;
        span.baseline += marker_shift;
    }

    jumps.append(&mut markers);

    let markers = jumps;

    let mut lyric_rows: Vec<Vec<(usize, usize)>> = vec![Vec::new(); system.voices.len()];

    for (group_index, group) in system.lyric_groups.iter().enumerate() {
        let Some(last_voice) = group
            .voices
            .iter()
            .filter_map(|voice| system.voices.iter().position(|item| item == voice))
            .max()
        else {
            continue;
        };

        for (verse_index, verse) in group.verses.iter().enumerate() {
            if !verse.is_empty() {
                lyric_rows[last_voice].push((group_index, verse_index));
            }
        }
    }

    let mut specs = Vec::new();
    let mut kinds = Vec::new();

    for (voice_index, voice) in system.voices.iter().enumerate() {
        specs.push(RowSpec {
            height: options.voice_row_height,
            gap_before: 0.0,
        });

        kinds.push(SystemRowKind::Voice(*voice));

        for &(group_index, verse_index) in &lyric_rows[voice_index] {
            specs.push(RowSpec {
                height: options.lyric_row_height,
                gap_before: 0.0,
            });
            kinds.push(SystemRowKind::Lyric {
                group_index,
                verse_index,
            });
        }
    }

    let column = layout_column(&specs);

    let rows = kinds
        .into_iter()
        .enumerate()
        .map(|(index, kind)| SystemRow {
            y: column.ys[index] + marker_shift + marker_height + dynamic_span_height,
            height: specs[index].height,
            kind,
        })
        .collect();

    SystemLayout {
        rows,
        markers,
        dynamic_spans,
        voltas,
        height: column.height
            + marker_height
            + volta_height
            + jump_offset
            + volta_top
            + dynamic_span_height,
    }
}

fn layout_jumps(system: &System, fonts: &Fonts, lead: f32) -> (Vec<PlacedMarker>, f32) {
    let xs = pulse_offsets(system);
    let usable_widths = system
        .pulses
        .iter()
        .map(|pulse| pulse_usable_width(pulse, fonts, lead))
        .collect::<Vec<_>>();
    let mut jumps = Vec::new();

    for marker in system
        .markers
        .iter()
        .filter(|marker| marker.kind == MarkerKind::Jump)
    {
        let font = marker_font(marker.kind, fonts);
        let width = match &marker.glyph {
            MarkerGlyph::Text(text) => font.width(text),
            MarkerGlyph::Segno => segno_width(fonts),
            MarkerGlyph::Fermata | MarkerGlyph::Staccato | MarkerGlyph::Marcato => {
                touch_width(&marker.glyph, fonts)
            }
        };

        jumps.push(PlacedMarker {
            kind: marker.kind,
            glyph: marker.glyph.clone(),
            width,
            anchor: TextAnchor::End,
            x: xs[marker.pulse] + marker.unit * usable_widths[marker.pulse] + lead,
            y: font.ascent(),
        });
    }

    if jumps.is_empty() {
        return (Vec::new(), 0.0);
    }

    spread_markers(&mut jumps);
    let height = fonts.marker_bold.line_height() + MARKER_BOTTOM_GAP;

    (jumps, height)
}

fn layout_voltas(system: &System, fonts: &Fonts, lead: f32) -> (Vec<PlacedVolta>, f32) {
    if system.voltas.is_empty() {
        return (Vec::new(), 0.0);
    }

    let xs = pulse_offsets(system);
    let text_y = fonts.marker_bold.ascent();
    let line_y = fonts.marker_bold.line_height() + VOLTA_TEXT_GAP;
    let mut voltas = Vec::new();

    for volta in &system.voltas {
        if volta.start >= volta.end {
            continue;
        }

        let start_x = xs[volta.start] + lead;
        let end_x = xs[volta.end - 1] + system.pulses[volta.end - 1].pulse_width;
        let text = match &volta.label {
            Some(label) => format!("{}. {label}", volta.number),
            None => format!("{}.", volta.number),
        };

        voltas.push(PlacedVolta {
            start_x,
            end_x,
            text,
            text_x: start_x + VOLTA_TEXT_INSET,
            text_y,
            line_y,
        });
    }

    if voltas.is_empty() {
        return (Vec::new(), 0.0);
    }

    let height = line_y + VOLTA_END_TICK + VOLTA_BOTTOM_GAP;

    (voltas, height)
}

fn layout_dynamic_spans(
    system: &System,
    fonts: &Fonts,
    lead: f32,
    dynamic_center: Option<f32>,
    marker_height: f32,
) -> (Vec<PlacedDynamicSpan>, f32) {
    if system.dynamic_spans.is_empty() {
        return (Vec::new(), 0.0);
    }

    let xs = pulse_offsets(system);
    let usable_widths = system
        .pulses
        .iter()
        .map(|pulse| pulse_usable_width(pulse, fonts, lead))
        .collect::<Vec<_>>();
    let row_height = fonts.dynamic.line_height().max(DYNAMIC_HAIRPIN_HEIGHT);
    let (center_y, height) = match dynamic_center {
        Some(center_y) => (center_y, 0.0),
        None => (
            marker_height + row_height / 2.0,
            row_height + DYNAMIC_SPAN_BOTTOM_GAP,
        ),
    };
    let baseline = center_y + (fonts.dynamic.ascent() - fonts.dynamic.descent()) / 2.0;
    let mut spans = Vec::new();

    for span in &system.dynamic_spans {
        let start_x = xs[span.start] + span.start_unit * usable_widths[span.start] + lead;
        let end_x = xs[span.end] + span.end_unit * usable_widths[span.end] + lead;

        if span.fits {
            let start_x = start_x + DYNAMIC_SPAN_INSET;
            let end_x = (end_x - DYNAMIC_SPAN_INSET).max(start_x + 1.0);

            spans.push(PlacedDynamicSpan {
                kind: span.kind,
                start_x,
                end_x,
                center_y,
                baseline,
                label: None,
                label_width: 0.0,
            });
        } else {
            let label = match span.kind {
                DynamicSpanKind::Crescendo => "cresc.",
                DynamicSpanKind::Decrescendo => "decresc.",
            };

            spans.push(PlacedDynamicSpan {
                kind: span.kind,
                start_x,
                end_x,
                center_y,
                baseline,
                label: Some(label.to_string()),
                label_width: fonts.dynamic.width(label),
            });
        }
    }

    (spans, height)
}

/// Place section markers above the note block. Time signatures get the
/// top lane; every other marker shares the lane below it.
fn layout_markers(
    system: &System,
    fonts: &Fonts,
    lead: f32,
    separate_jumps: bool,
) -> (Vec<PlacedMarker>, f32) {
    if system.markers.is_empty() {
        return (Vec::new(), 0.0);
    }

    let xs = pulse_offsets(system);
    let usable_widths = system
        .pulses
        .iter()
        .map(|pulse| pulse_usable_width(pulse, fonts, lead))
        .collect::<Vec<_>>();

    let note_center = |pulse_id: usize, unit: f32| -> Option<f32> {
        let pulse = system.pulses.get(pulse_id)?;
        let usable_width = usable_widths[pulse_id];

        for voice in &system.voices {
            let Some(voice_pulse) = find_voice(pulse, *voice) else {
                continue;
            };

            let total = voice_pulse
                .columns
                .iter()
                .map(|column| column.duration as u32)
                .sum::<u32>()
                .max(1);
            let mut cursor = 0u32;

            for (index, column) in voice_pulse.columns.iter().enumerate() {
                let start = cursor as f32 / total as f32;
                cursor += column.duration as u32;
                let end = cursor as f32 / total as f32;
                let is_last = index + 1 == voice_pulse.columns.len();

                if unit + MARKER_SAME_NOTE_EPSILON >= start
                    && (unit < end - MARKER_SAME_NOTE_EPSILON
                        || (is_last && unit <= end + MARKER_SAME_NOTE_EPSILON))
                {
                    if matches!(column.note, NoteKind::EmptyNote) {
                        break;
                    }

                    let (text, _) = note_text(&column.note);
                    let glyph_width = fonts.solfa.width(&text);
                    let leading = match column.note {
                        NoteKind::ProlongedNote => lead + prolonged_leading(fonts),
                        _ => lead,
                    };

                    return Some(xs[pulse_id] + start * usable_width + leading + glyph_width / 2.0);
                }
            }
        }

        None
    };

    let mut ordered = system
        .markers
        .iter()
        .filter(|marker| !separate_jumps || marker.kind != MarkerKind::Jump)
        .collect::<Vec<_>>();

    ordered.sort_by(|lhs, rhs| {
        lhs.pulse
            .cmp(&rhs.pulse)
            .then(lhs.unit.total_cmp(&rhs.unit))
    });

    let mut groups: Vec<Vec<&SystemMarker>> = Vec::new();

    for marker in ordered {
        if let Some(group) = groups.last_mut() {
            let first = group[0];

            if first.pulse == marker.pulse
                && (first.unit - marker.unit).abs() < MARKER_SAME_NOTE_EPSILON
            {
                group.push(marker);
                continue;
            }
        }

        groups.push(vec![marker]);
    }

    let mut rows: Vec<Vec<PlacedMarker>> = Vec::new();

    for mut group in groups {
        group.sort_by_key(|marker| marker_priority(marker.kind));

        for (row, marker) in group.into_iter().enumerate() {
            let font = marker_font(marker.kind, fonts);
            let width = match &marker.glyph {
                MarkerGlyph::Text(text) => font.width(text),
                MarkerGlyph::Segno => segno_width(fonts),
                MarkerGlyph::Fermata | MarkerGlyph::Staccato | MarkerGlyph::Marcato => {
                    touch_width(&marker.glyph, fonts)
                }
            };
            let fallback_x = xs[marker.pulse] + marker.unit * usable_widths[marker.pulse] + lead;
            let x = match marker.kind {
                MarkerKind::Dynamic | MarkerKind::Touch => {
                    note_center(marker.pulse, marker.unit).unwrap_or(fallback_x)
                }
                MarkerKind::Mark if matches!(&marker.glyph, MarkerGlyph::Segno) => {
                    note_center(marker.pulse, marker.unit).unwrap_or(fallback_x)
                }
                _ => fallback_x,
            };

            let placed = PlacedMarker {
                kind: marker.kind,
                glyph: marker.glyph.clone(),
                width,
                x,
                y: 0.0,
                anchor: match marker.kind {
                    MarkerKind::TimeSignature => TextAnchor::Middle,
                    MarkerKind::Key | MarkerKind::Tempo | MarkerKind::TempoNumber => {
                        TextAnchor::Start
                    }
                    MarkerKind::Dynamic | MarkerKind::Touch => TextAnchor::Middle,
                    MarkerKind::Jump => TextAnchor::End,
                    MarkerKind::Mark if marker.unit >= 0.5 => TextAnchor::End,
                    MarkerKind::Mark => TextAnchor::Middle,
                },
            };

            if rows.len() <= row {
                rows.resize_with(row + 1, Vec::new);
            }

            rows[row].push(placed);
        }
    }

    for row in &mut rows {
        spread_markers(row);
    }

    let mut top = 0.0;

    for row in &mut rows {
        if row.is_empty() {
            continue;
        }

        let row_height = row
            .iter()
            .map(|marker| marker_font(marker.kind, fonts).line_height())
            .fold(0.0, f32::max);

        for marker in row {
            let font = marker_font(marker.kind, fonts);
            marker.y = top + (row_height - font.line_height()) / 2.0 + font.ascent();
        }

        top += row_height;
    }

    if top == 0.0 {
        return (Vec::new(), 0.0);
    }

    let markers = rows.into_iter().flatten().collect();
    let height = top + MARKER_BOTTOM_GAP;

    (markers, height)
}

pub struct SystemLayout {
    pub rows: Vec<SystemRow>,
    pub markers: Vec<PlacedMarker>,
    pub dynamic_spans: Vec<PlacedDynamicSpan>,
    pub voltas: Vec<PlacedVolta>,
    pub height: f32,
}

pub struct PlacedDynamicSpan {
    pub kind: DynamicSpanKind,
    pub start_x: f32,
    pub end_x: f32,
    pub center_y: f32,
    pub baseline: f32,
    pub label: Option<String>,
    pub label_width: f32,
}

pub struct PlacedVolta {
    pub start_x: f32,
    pub end_x: f32,
    pub text: String,
    pub text_x: f32,
    pub text_y: f32,
    pub line_y: f32,
}

#[derive(Clone)]
pub struct PlacedMarker {
    pub kind: MarkerKind,
    pub glyph: MarkerGlyph,
    pub width: f32,
    pub anchor: TextAnchor,
    pub x: f32,
    pub y: f32,
}

pub struct SystemRow {
    pub y: f32,
    pub height: f32,
    pub kind: SystemRowKind,
}

pub enum SystemRowKind {
    Voice(Voice),
    Lyric {
        group_index: usize,
        verse_index: usize,
    },
}

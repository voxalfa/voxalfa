use crate::{
    fonts::Fonts,
    model::{MarkerKind, System, VoiceInstance},
    options::RenderOptions,
};

use super::{
    constants::*,
    dynamics::{PlacedDynamicSpan, layout_dynamic_spans},
    markers::{PlacedMarker, layout_markers},
    voltas::{PlacedVolta, layout_voltas},
};

pub fn layout_system(
    system: &System,
    fonts: &Fonts,
    lead: f32,
    options: &RenderOptions,
) -> SystemLayout {
    let (mut voltas, volta_height) = layout_voltas(system, fonts, lead, options);
    let mut marker_layout = layout_markers(system, fonts, lead, options, &voltas, volta_height);

    shift_voltas(&mut voltas, marker_layout.volta_offset);

    let lyric_rows = collect_lyric_rows(system);

    let (rows, column) = layout_rows(
        system,
        fonts,
        options,
        &marker_layout.markers,
        &lyric_rows,
        marker_layout.height,
    );

    place_voice_dynamics(&mut marker_layout.markers, &rows, system, fonts);

    let dynamic_spans = layout_dynamic_spans(
        system,
        fonts,
        lead,
        marker_layout.height,
        &marker_layout.markers,
        options,
    );

    let height = column.height + marker_layout.height;

    SystemLayout {
        rows,
        markers: marker_layout.markers,
        dynamic_spans,
        voltas,
        height,
    }
}

fn shift_voltas(voltas: &mut [PlacedVolta], offset: f32) {
    for volta in voltas {
        volta.text_y += offset;
        volta.line_y += offset;
    }
}

fn collect_lyric_rows(system: &System) -> Vec<Vec<(usize, usize)>> {
    let mut rows = vec![Vec::new(); system.voices.len()];

    for (group_index, group) in system.lyric_groups.iter().enumerate() {
        let last_voice = group
            .voices
            .iter()
            .filter_map(|voice| system.voices.iter().position(|item| item == voice))
            .max();

        if let Some(last_voice) = last_voice {
            for (verse_index, verse) in group.verses.iter().enumerate() {
                if !verse.is_empty() {
                    rows[last_voice].push((group_index, verse_index));
                }
            }
        };
    }

    rows
}

fn layout_rows(
    system: &System,
    fonts: &Fonts,
    options: &RenderOptions,
    markers: &[PlacedMarker],
    lyric_rows: &[Vec<(usize, usize)>],
    top: f32,
) -> (Vec<SystemRow>, ColumnLayout) {
    let elements = &options.elements;
    let note_scale = fonts.text.note.size() / options.solfa_font_size.max(1.0);
    let lyric_scale = fonts.text.lyric.size() / options.lyric_font_size.max(1.0);

    let voice_height =
        options.voice_row_height * note_scale + elements.note.top() + elements.note.bottom();

    let lyric_height =
        options.lyric_row_height * lyric_scale + elements.lyrics.top() + elements.lyrics.bottom();

    let dynamic_height = fonts.label.dynamic.line_height()
        + elements.dynamic.top()
        + elements.dynamic.bottom()
        + fonts.label.dynamic.size() * DYNAMIC_VOICE_GAP;

    let global_voice = system.voices.first().map(|voice| voice.id);

    let mut specs = Vec::new();
    let mut kinds = Vec::new();

    for (voice_index, voice) in system.voices.iter().enumerate() {
        let has_dynamic = has_voice_dynamic(markers, *voice, global_voice);

        specs.push(RowSpec {
            height: voice_height,
            gap_before: if has_dynamic { dynamic_height } else { 0.0 },
        });

        kinds.push(SystemRowKind::Voice(*voice));

        for &(group_index, verse_index) in &lyric_rows[voice_index] {
            specs.push(RowSpec {
                height: lyric_height,
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
            y: column.ys[index] + top,
            height: specs[index].height,
            kind,
        })
        .collect();

    (rows, column)
}

fn has_voice_dynamic(
    markers: &[PlacedMarker],
    voice: VoiceInstance,
    global_voice: Option<usize>,
) -> bool {
    Some(voice.id) != global_voice
        && markers.iter().any(|marker| {
            marker.kind == MarkerKind::Dynamic && marker.voice.map(|item| item.id) == Some(voice.id)
        })
}

fn place_voice_dynamics(
    markers: &mut [PlacedMarker],
    rows: &[SystemRow],
    system: &System,
    fonts: &Fonts,
) {
    let global_voice = system.voices.first().map(|voice| voice.id);
    let offset = fonts.label.dynamic.size() * DYNAMIC_VOICE_GAP;

    let dynamics_markers = markers
        .iter_mut()
        .filter(|marker| marker.kind == MarkerKind::Dynamic);

    for marker in dynamics_markers {
        let Some(voice) = marker.voice else {
            continue;
        };

        if Some(voice.id) == global_voice {
            continue;
        }

        let Some(row) = rows.iter().find(
            |row| matches!(row.kind, SystemRowKind::Voice(current) if current.id == voice.id),
        ) else {
            continue;
        };

        marker.y = row.y - offset;
    }
}

fn layout_column(specs: &[RowSpec]) -> ColumnLayout {
    let mut ys = Vec::with_capacity(specs.len());
    let mut y = 0.0;

    for spec in specs {
        y += spec.gap_before;
        ys.push(y);
        y += spec.height;
    }

    ColumnLayout { ys, height: y }
}

#[derive(Debug)]
struct RowSpec {
    height: f32,
    gap_before: f32,
}

#[derive(Debug)]
pub struct SystemLayout {
    pub rows: Vec<SystemRow>,
    pub markers: Vec<PlacedMarker>,
    pub dynamic_spans: Vec<PlacedDynamicSpan>,
    pub voltas: Vec<PlacedVolta>,
    pub height: f32,
}

#[derive(Debug)]
pub struct SystemRow {
    pub y: f32,
    pub height: f32,
    pub kind: SystemRowKind,
}

#[derive(Debug)]
pub enum SystemRowKind {
    Voice(VoiceInstance),
    Lyric {
        group_index: usize,
        verse_index: usize,
    },
}

#[derive(Debug)]
pub struct ColumnLayout {
    pub ys: Vec<f32>,
    pub height: f32,
}

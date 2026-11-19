use voxalfa_core::data_types::Voice;

use crate::{fonts::Fonts, model::System, options::RenderOptions};

use super::{
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
    let elements = &options.elements;

    let (mut voltas, volta_height) = layout_voltas(system, fonts, lead, options);
    let marker_layout = layout_markers(system, fonts, lead, options, &voltas, volta_height);

    for volta in &mut voltas {
        volta.text_y += marker_layout.volta_offset;
        volta.line_y += marker_layout.volta_offset;
    }

    let mut lyric_rows: Vec<Vec<(usize, usize)>> = vec![Vec::new(); system.voices.len()];

    for (group_index, group) in system.lyric_groups.iter().enumerate() {
        let last_voice = group
            .voices
            .iter()
            .filter_map(|voice| system.voices.iter().position(|item| item == voice))
            .max();

        if let Some(last_voice) = last_voice {
            for (verse_index, verse) in group.verses.iter().enumerate() {
                if !verse.is_empty() {
                    lyric_rows[last_voice].push((group_index, verse_index));
                }
            }
        };
    }

    let note_scale = fonts.text.note.size() / options.solfa_font_size.max(1.0);
    let lyric_scale = fonts.text.lyric.size() / options.lyric_font_size.max(1.0);
    let note_style = &elements.note;
    let lyric_style = &elements.lyrics;

    let voice_height =
        options.voice_row_height * note_scale + note_style.top() + note_style.bottom();

    let lyric_height =
        options.lyric_row_height * lyric_scale + lyric_style.top() + lyric_style.bottom();

    let mut heights = Vec::new();
    let mut kinds = Vec::new();

    for (voice_index, voice) in system.voices.iter().enumerate() {
        heights.push(voice_height);
        kinds.push(SystemRowKind::Voice(*voice));

        for &(group_index, verse_index) in &lyric_rows[voice_index] {
            heights.push(lyric_height);
            kinds.push(SystemRowKind::Lyric {
                group_index,
                verse_index,
            });
        }
    }

    let column = layout_column(&heights);

    let rows = kinds
        .into_iter()
        .enumerate()
        .map(|(index, kind)| SystemRow {
            y: column.ys[index] + marker_layout.height,
            height: heights[index],
            kind,
        })
        .collect::<Vec<_>>();

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

fn layout_column(heights: &[f32]) -> ColumnLayout {
    let mut ys = Vec::with_capacity(heights.len());
    let mut y = 0.0;

    for &height in heights {
        ys.push(y);
        y += height;
    }

    ColumnLayout { ys, height: y }
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
    Voice(Voice),
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

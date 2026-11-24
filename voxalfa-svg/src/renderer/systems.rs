use crate::{
    RenderOptions,
    fonts::Fonts,
    model::{System, VoiceInstance},
    renderer::markers::PlacedMarker,
};

use super::{
    constants::*,
    dynamics::{has_voice_dynamic, place_voice_dynamics},
    markers::{MarkerLayout, layout_markers},
    spans::{PlacedSpan, SpanPlan, SpanPlanner, layout_spans},
    voltas::{PlacedVolta, layout_voltas, place_voltas},
};

pub struct LayoutContext<'a, 'b> {
    pub system: &'a System<'b>,
    pub fonts: &'a Fonts,
    pub lead: f32,
    pub options: &'a RenderOptions,
}

pub fn layout_system(ctx: &LayoutContext) -> SystemLayout {
    let mut marker_layout = layout_markers(ctx);
    let mut voltas = layout_voltas(ctx);

    place_voltas(ctx, &mut marker_layout, &mut voltas);

    let lyric_rows = collect_lyric_rows(ctx.system);
    let span_plan = SpanPlanner::new(ctx, &marker_layout).plan();
    let span_offset = span_plan.global_height() + span_plan.top_height();

    marker_layout.shift(span_offset);

    let (rows, column) = layout_rows(ctx, &marker_layout, &lyric_rows, &span_plan);

    place_voice_dynamics(ctx, &mut marker_layout, &rows);

    let spans = layout_spans(ctx, &marker_layout, &rows, &span_plan);
    let height = column.height + marker_layout.height;

    SystemLayout {
        rows,
        markers: marker_layout.markers,
        spans,
        voltas,
        height,
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
    ctx: &LayoutContext,
    marker_layout: &MarkerLayout,
    lyric_rows: &[Vec<(usize, usize)>],
    span_plan: &SpanPlan,
) -> (Vec<SystemRow>, ColumnLayout) {
    let elements = &ctx.options.elements;
    let note_scale = ctx.fonts.text.note.size() / ctx.options.solfa_font_size.max(1.0);
    let lyric_scale = ctx.fonts.text.lyric.size() / ctx.options.lyric_font_size.max(1.0);

    let voice_height =
        ctx.options.voice_row_height * note_scale + elements.note.top() + elements.note.bottom();

    let lyric_height = ctx.options.lyric_row_height * lyric_scale
        + elements.lyrics.top()
        + elements.lyrics.bottom();

    let dynamic_height = ctx.fonts.label.dynamic.line_height()
        + elements.dynamic.top()
        + elements.dynamic.bottom()
        + ctx.fonts.label.dynamic.size() * DYNAMIC_VOICE_GAP;

    let global_voice = ctx.system.voices.first().map(|voice| voice.id);

    let mut specs = Vec::new();
    let mut kinds = Vec::new();

    for (voice_index, voice) in ctx.system.voices.iter().enumerate() {
        let has_dynamic = has_voice_dynamic(&marker_layout.markers, *voice, global_voice);

        specs.push(RowSpec {
            height: voice_height,
            gap_before: if has_dynamic { dynamic_height } else { 0.0 }
                + span_plan.local_height(voice.id),
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
            y: column.row_offsets[index] + marker_layout.height,
            height: specs[index].height,
            kind,
        })
        .collect();

    (rows, column)
}

fn layout_column(specs: &[RowSpec]) -> ColumnLayout {
    let mut row_offsets = Vec::with_capacity(specs.len());
    let mut y = 0.0;

    for spec in specs {
        y += spec.gap_before;
        row_offsets.push(y);
        y += spec.height;
    }

    ColumnLayout {
        row_offsets,
        height: y,
    }
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
    pub spans: Vec<PlacedSpan>,
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
    pub row_offsets: Vec<f32>,
    pub height: f32,
}

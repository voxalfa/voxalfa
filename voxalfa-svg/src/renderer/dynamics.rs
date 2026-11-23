use crate::{
    model::{MarkerKind, SpanKind, VoiceInstance},
    renderer::systems::LayoutContext,
};

use super::{
    constants::*,
    geometry::SystemGeometry,
    markers::{MarkerLayout, PlacedMarker, label_at},
    systems::{SystemRow, SystemRowKind},
};

pub fn has_voice_dynamic(
    markers: &[PlacedMarker],
    voice: VoiceInstance,
    global_voice: Option<usize>,
) -> bool {
    Some(voice.id) != global_voice
        && markers.iter().any(|marker| {
            marker.kind == MarkerKind::Dynamic && marker.voice.map(|item| item.id) == Some(voice.id)
        })
}

pub fn place_voice_dynamics(ctx: &LayoutContext, layout: &mut MarkerLayout, rows: &[SystemRow]) {
    let global_voice = ctx.system.voices.first().map(|voice| voice.id);
    let offset = ctx.fonts.label.dynamic.size() * DYNAMIC_VOICE_GAP;

    let dynamics_marker = layout
        .markers
        .iter_mut()
        .filter(|marker| marker.kind == MarkerKind::Dynamic);

    for marker in dynamics_marker {
        let Some(voice) = marker.voice else {
            continue;
        };

        if Some(voice.id) == global_voice {
            continue;
        }

        let Some(row) = voice_row(rows, voice.id) else {
            continue;
        };

        marker.y = row.y - offset;
    }
}

fn voice_row(rows: &[SystemRow], id: usize) -> Option<&SystemRow> {
    rows.iter()
        .find(|row| matches!(row.kind, SystemRowKind::Voice(current) if current.id == id))
}

pub fn layout_spans(ctx: &LayoutContext, layout: &MarkerLayout) -> Vec<PlacedSpan> {
    if ctx.system.spans.is_empty() {
        return Vec::new();
    }

    let style = &ctx.options.elements.dynamic;
    let geometry = SystemGeometry::new(ctx.system, ctx.fonts, ctx.lead);
    let markers = &layout.markers;

    let dynamic_center = markers
        .iter()
        .find(|marker| marker.kind == MarkerKind::Dynamic && marker.voice.is_none())
        .map(|marker| ctx.fonts.label.dynamic.line_center(marker.y));

    let voice_centers = ctx
        .system
        .voices
        .iter()
        .filter_map(|voice| {
            let marker = markers.iter().find(|marker| {
                marker.kind == MarkerKind::Dynamic
                    && marker.voice.map(|item| item.id) == Some(voice.id)
            })?;

            Some((*voice, ctx.fonts.label.dynamic.line_center(marker.y)))
        })
        .collect::<Vec<_>>();

    let dynamic_row_height = ctx
        .fonts
        .label
        .dynamic
        .line_height()
        .max(ctx.fonts.label.dynamic.size() * DYNAMIC_HAIRPIN_HEIGHT);

    let fallback_center =
        layout.height - ctx.fonts.label.key.size() * MARKER_BOTTOM_GAP - dynamic_row_height / 2.0
            + style.top()
            - style.bottom();

    let mut spans = Vec::new();
    let label_gap = ctx.fonts.label.dynamic.size() * DYNAMIC_SPAN_LABEL_GAP;

    for span in &ctx.system.spans {
        let center_y = span
            .voice
            .and_then(|voice| {
                voice_centers
                    .iter()
                    .find(|(current, _)| current.id == voice.id)
                    .map(|(_, center)| *center)
            })
            .or(dynamic_center)
            .unwrap_or(fallback_center);

        let label_font = if span.kind.is_tempo() {
            &ctx.fonts.label.tempo
        } else {
            &ctx.fonts.label.dynamic
        };

        let baseline = center_y + (label_font.ascent() - label_font.descent()) / 2.0;

        let fallback_start_x = geometry.note_x(span.start, span.start_unit) + style.left();

        let start_x = geometry
            .note_bounds(span.start, span.start_unit, span.voice)
            .map(|(start, _)| start + style.left())
            .unwrap_or(fallback_start_x);

        let fallback_end_x = geometry.note_x(span.end, span.end_unit) - style.right();

        let end_x = geometry
            .note_bounds(span.end, span.end_unit, span.voice)
            .map(|(_, end)| end - style.right())
            .unwrap_or(fallback_end_x);

        let start_x = match label_at(markers, span.start, span.start_unit, span.voice) {
            Some(marker) => {
                let (_, right) = marker.horizontal_bounds();
                (right + label_gap).max(start_x)
            }
            None => start_x,
        };

        let end_x = match label_at(markers, span.end, span.end_unit, span.voice) {
            Some(marker) => {
                let (left, _) = marker.horizontal_bounds();
                (left - label_gap).min(end_x)
            }
            None => end_x,
        };

        let label = if span.kind.is_tempo() || (!span.fits && !span.continued) {
            Some(format!("{}.", span.kind.label()))
        } else {
            None
        };

        let label_width = label
            .as_ref()
            .map(|label| label_font.width(label))
            .unwrap_or_default();

        spans.push(PlacedSpan {
            kind: span.kind,
            start_x,
            end_x: end_x.max(start_x + 1.0),
            center_y,
            baseline,
            label_width,
            label,
            continuation: !span.fits && span.continued,
        });
    }

    spans
}

#[derive(Debug)]
pub struct PlacedSpan {
    pub kind: SpanKind,
    pub start_x: f32,
    pub end_x: f32,
    pub center_y: f32,
    pub baseline: f32,
    pub label: Option<String>,
    pub label_width: f32,
    pub continuation: bool,
}

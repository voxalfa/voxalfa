use crate::{
    fonts::Fonts,
    model::{DynamicSpanKind, MarkerKind, System},
    options::RenderOptions,
};

use super::{
    constants::*,
    geometry::{pulse_offsets, pulse_usable_width},
    markers::{PlacedMarker, label_at, marker_bounds},
};

pub fn layout_dynamic_spans(
    system: &System,
    fonts: &Fonts,
    lead: f32,
    marker_height: f32,
    markers: &[PlacedMarker],
    options: &RenderOptions,
) -> Vec<PlacedDynamicSpan> {
    if system.dynamic_spans.is_empty() {
        return Vec::new();
    }

    let style = &options.elements.dynamic;
    let xs = pulse_offsets(system);

    let usable_widths = system
        .pulses
        .iter()
        .map(|pulse| pulse_usable_width(pulse, fonts, lead))
        .collect::<Vec<_>>();

    let dynamic_center = markers
        .iter()
        .find(|marker| marker.kind == MarkerKind::Dynamic && marker.voice.is_none())
        .map(|marker| {
            marker.y - (fonts.label.dynamic.ascent() - fonts.label.dynamic.descent()) / 2.0
        });

    let voice_centers = system
        .voices
        .iter()
        .filter_map(|voice| {
            let marker = markers.iter().find(|marker| {
                marker.kind == MarkerKind::Dynamic && marker.voice == Some(*voice)
            })?;

            Some((
                *voice,
                marker.y - (fonts.label.dynamic.ascent() - fonts.label.dynamic.descent()) / 2.0,
            ))
        })
        .collect::<Vec<_>>();

    let dynamic_row_height = fonts
        .label
        .dynamic
        .line_height()
        .max(fonts.label.dynamic.size() * DYNAMIC_HAIRPIN_HEIGHT);

    let fallback_center =
        marker_height - fonts.label.key.size() * MARKER_BOTTOM_GAP - dynamic_row_height / 2.0
            + style.top()
            - style.bottom();

    let mut spans = Vec::new();
    let label_gap = fonts.label.dynamic.size() * DYNAMIC_SPAN_LABEL_GAP;

    for span in &system.dynamic_spans {
        let center_y = span
            .voice
            .and_then(|voice| {
                voice_centers
                    .iter()
                    .find(|(current, _)| *current == voice)
                    .map(|(_, center)| *center)
            })
            .or(dynamic_center)
            .unwrap_or(fallback_center);

        let baseline =
            center_y + (fonts.label.dynamic.ascent() - fonts.label.dynamic.descent()) / 2.0;

        let start_x =
            xs[span.start] + span.start_unit * usable_widths[span.start] + lead + style.left();

        let end_unit = (span.end_unit + span.end_duration).min(1.0);
        let end_x = xs[span.end] + end_unit * usable_widths[span.end] + lead - style.right();

        let start_x = match label_at(markers, span.start, span.start_unit, span.voice) {
            Some(marker) => {
                let (_, right) = marker_bounds(marker);
                (right + label_gap).max(start_x)
            }
            None => start_x,
        };

        let end_x = match label_at(markers, span.end, span.end_unit, span.voice) {
            Some(marker) => {
                let (left, _) = marker_bounds(marker);
                (left - label_gap).min(end_x)
            }
            None => end_x,
        };

        let label = (!span.fits && !span.continued).then(|| match span.kind {
            DynamicSpanKind::Crescendo => "cresc.".to_string(),
            DynamicSpanKind::Decrescendo => "decresc.".to_string(),
        });

        let label_width = label
            .as_ref()
            .map(|label| fonts.label.dynamic.width(label))
            .unwrap_or_default();

        spans.push(PlacedDynamicSpan {
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
pub struct PlacedDynamicSpan {
    pub kind: DynamicSpanKind,
    pub start_x: f32,
    pub end_x: f32,
    pub center_y: f32,
    pub baseline: f32,
    pub label: Option<String>,
    pub label_width: f32,
    pub continuation: bool,
}

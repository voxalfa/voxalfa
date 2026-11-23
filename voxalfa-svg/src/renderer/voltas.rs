use crate::{
    fonts::{FontMetrics, Fonts},
    renderer::systems::LayoutContext,
};

use super::{
    constants::*,
    markers::{MarkerLayout, PlacedMarker},
};

pub fn layout_voltas(ctx: &LayoutContext) -> Vec<PlacedVolta> {
    let style = &ctx.options.elements.volta;
    let mut voltas = Vec::new();

    for volta in &ctx.system.voltas {
        if volta.start >= volta.end {
            continue;
        }

        let start_x = ctx.system.pulse_offsets[volta.start] + ctx.lead;
        let end_x =
            ctx.system.pulse_offsets[volta.end - 1] + ctx.system.pulses[volta.end - 1].pulse_width;

        let text = if volta.open_start {
            String::new()
        } else {
            match &volta.label {
                Some(label) => format!("{}. {label}", volta.number),
                None => format!("{}.", volta.number),
            }
        };

        let text_width = ctx.fonts.label.volta.width(&text);
        let padding = ctx.fonts.label.volta.size() * VOLTA_TEXT_INSET;

        let text_x = (start_x + style.left() + padding)
            .min((end_x - padding - text_width).max(start_x + padding));

        voltas.push(PlacedVolta {
            start_x,
            end_x,
            label: text,
            label_x: text_x,
            label_width: text_width,
            label_baseline: 0.0,
            line_y: 0.0,
            open_start: volta.open_start,
            open_end: volta.open_end,
        });
    }

    voltas
}

pub fn place_voltas(ctx: &LayoutContext, layout: &mut MarkerLayout, voltas: &mut [PlacedVolta]) {
    if voltas.is_empty() {
        return;
    }

    let font = &ctx.fonts.label.volta;
    let gap = font.size() * VOLTA_MARKER_GAP;

    pad_markers(layout, voltas, font.size() * VOLTA_TEXT_INSET);

    let start_y = layout.bottom_baseline().unwrap_or_else(|| font.ascent());

    let label_baselines = voltas
        .iter()
        .map(|volta| resolve_label_baseline(volta, &layout.markers, ctx.fonts, gap, start_y))
        .collect::<Vec<_>>();

    let shift = label_baselines
        .iter()
        .map(|label_y| -bar_baseline(*label_y, font, gap))
        .fold(0.0, f32::max);

    if shift > 0.0 {
        layout.shift(shift);
    }

    let label_bottom = label_baselines
        .iter()
        .map(|label_y| label_y + shift + font.descent())
        .fold(0.0, f32::max);

    layout.ensure_height(label_bottom + ctx.fonts.label.key.size() * MARKER_BOTTOM_GAP);

    for (volta, label_y) in voltas.iter_mut().zip(label_baselines) {
        volta.label_baseline = label_y + shift;
        volta.line_y = bar_baseline(volta.label_baseline, font, gap);
    }
}

fn resolve_label_baseline(
    volta: &PlacedVolta,
    markers: &[PlacedMarker],
    fonts: &Fonts,
    gap: f32,
    start_y: f32,
) -> f32 {
    let font = &fonts.label.volta;
    let label_left = volta.label_x;
    let label_right = volta.label_x + volta.label_width;
    let mut label_y = start_y;

    loop {
        let label_top = label_y - font.ascent();
        let label_bottom = label_y + font.descent();
        let mut offset: f32 = 0.0;

        for marker in markers {
            let (left, right) = marker.horizontal_bounds();
            let (top, bottom) = marker.vertical_bounds(fonts);

            if right > volta.start_x && left < volta.end_x && top < label_top {
                offset = offset.min(top - label_top); // vertical marker collision
            }

            if volta.open_start {
                continue;
            }

            if right < label_left || left > label_right {
                continue; // no horizontal label collision
            }

            if label_bottom + gap > top && label_top < bottom {
                offset = offset.min(top - font.descent() - gap - label_y); // vertical label collision
            }
        }

        if offset >= 0.0 {
            break;
        }

        label_y += offset;
    }

    label_y
}

fn pad_markers(layout: &mut MarkerLayout, voltas: &[PlacedVolta], padding: f32) {
    for volta in voltas {
        let padded_end = volta.end_x - padding;

        for marker in &mut layout.markers {
            let (left, right) = marker.horizontal_bounds();

            if left < volta.end_x && right > padded_end {
                marker.x -= right - padded_end;
            }
        }
    }
}

fn bar_baseline(label_y: f32, font: &FontMetrics, gap: f32) -> f32 {
    label_y - font.ascent() - gap
}

#[derive(Debug)]
pub struct PlacedVolta {
    pub start_x: f32,
    pub end_x: f32,
    pub label: String,
    pub label_x: f32,
    pub label_width: f32,
    pub label_baseline: f32,
    pub line_y: f32,
    pub open_start: bool,
    pub open_end: bool,
}

use crate::{fonts::FontMetrics, renderer::context::LayoutContext};

use super::{collision::Rect, constants::*, markers::MarkerLayout};

pub fn layout_voltas(ctx: &LayoutContext) -> Vec<PlacedVolta> {
    let style = &ctx.options.elements.volta;
    let pulse_offsets = &ctx.system.pulse_offsets;

    let mut voltas = Vec::new();

    for volta in &ctx.system.voltas {
        if volta.start >= volta.end {
            continue;
        }

        let start_x = pulse_offsets[volta.start] + ctx.lead;
        let end_x = pulse_offsets[volta.end - 1] + ctx.system.pulses[volta.end - 1].pulse_width;

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
    let bracket_half = font.size() * 0.05;

    pad_markers(layout, voltas, font.size() * VOLTA_TEXT_INSET);

    let start_y = layout.bottom_baseline().unwrap_or_else(|| font.ascent());

    let mut obstacles = layout
        .markers
        .iter()
        .map(|marker| marker.rect(ctx.fonts))
        .collect::<Vec<_>>();

    let mut baselines = Vec::with_capacity(voltas.len());
    let mut placed_rects = Vec::new();

    for volta in voltas.iter() {
        let mut baseline = start_y;

        loop {
            let (label_rect, bracket_rect) = volta.rects(baseline, font, gap, bracket_half);
            let mut shift: f32 = 0.0;

            for obstacle in &obstacles {
                if label_rect.intersects(obstacle) {
                    shift = shift.max(label_rect.bottom - obstacle.top + gap);
                }

                if bracket_rect.intersects(obstacle) {
                    shift = shift.max(bracket_rect.bottom - obstacle.top + gap);
                }
            }

            if shift <= 0.0 {
                break;
            }

            baseline -= shift;
        }

        let (label_rect, bracket_rect) = volta.rects(baseline, font, gap, bracket_half);

        obstacles.extend([label_rect, bracket_rect]);
        placed_rects.extend([label_rect, bracket_rect]);
        baselines.push(baseline);
    }

    let min_top = placed_rects
        .iter()
        .map(|rect| rect.top)
        .fold(f32::INFINITY, f32::min);

    let shift = (-min_top).max(0.0);

    if shift > 0.0 {
        layout.shift(shift);
    }

    let label_bottom = placed_rects
        .iter()
        .map(|rect| rect.bottom + shift)
        .fold(0.0, f32::max);

    layout.ensure_height(label_bottom + ctx.fonts.label.key.size() * MARKER_BOTTOM_GAP);

    for (volta, baseline) in voltas.iter_mut().zip(baselines) {
        volta.label_baseline = baseline + shift;
        volta.line_y = bar_baseline(volta.label_baseline, font, gap);
    }
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

impl PlacedVolta {
    fn rects(
        &self,
        baseline: f32,
        font: &FontMetrics,
        gap: f32,
        bracket_half: f32,
    ) -> (Rect, Rect) {
        let label = Rect {
            left: self.label_x,
            right: self.label_x + self.label_width,
            top: baseline - font.ascent(),
            bottom: baseline + font.descent(),
        };

        let line_y = bar_baseline(baseline, font, gap);

        let bracket = Rect {
            left: self.start_x,
            right: self.end_x,
            top: line_y - bracket_half,
            bottom: line_y + bracket_half,
        };

        (label, bracket)
    }
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

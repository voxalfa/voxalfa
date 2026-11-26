use std::collections::HashMap;

use crate::{
    model::{MarkerKind, SpanKind, SystemSpan},
    renderer::context::LayoutContext,
};

use super::{
    collision::Rect,
    constants::*,
    dynamics::voice_row,
    geometry::SystemGeometry,
    markers::{MarkerLayout, PlacedMarker},
    systems::SystemRow,
};

#[derive(Debug)]
pub struct SpanPlan {
    row_height: f32,
    global_lanes: usize,
    top_lanes: usize,
    local_lanes: HashMap<usize, usize>,
    lanes: HashMap<usize, usize>,
}

impl SpanPlan {
    pub fn row_height(&self) -> f32 {
        self.row_height
    }

    pub fn global_height(&self) -> f32 {
        self.row_height * self.global_lanes as f32
    }

    pub fn top_height(&self) -> f32 {
        self.row_height * self.top_lanes as f32
    }

    pub fn local_height(&self, voice_id: usize) -> f32 {
        self.local_lanes.get(&voice_id).copied().unwrap_or_default() as f32 * self.row_height
    }

    fn lane(&self, span_index: usize) -> usize {
        self.lanes.get(&span_index).copied().unwrap_or_default()
    }
}

pub struct SpanPlanner<'a, 'b> {
    ctx: &'a LayoutContext<'a, 'b>,
    markers: &'a MarkerLayout,
    global_lanes: usize,
    top_lanes: usize,
    local_lanes: HashMap<usize, usize>,
    lanes: HashMap<usize, usize>,
}

impl<'a, 'b> SpanPlanner<'a, 'b> {
    pub fn new(ctx: &'a LayoutContext<'a, 'b>, markers: &'a MarkerLayout) -> Self {
        Self {
            ctx,
            markers,
            global_lanes: 0,
            top_lanes: 0,
            local_lanes: HashMap::new(),
            lanes: HashMap::new(),
        }
    }

    pub fn plan(mut self) -> SpanPlan {
        self.plan_global();
        self.plan_local();

        SpanPlan {
            row_height: self.row_height(),
            global_lanes: self.global_lanes,
            top_lanes: self.top_lanes,
            local_lanes: self.local_lanes,
            lanes: self.lanes,
        }
    }

    fn row_height(&self) -> f32 {
        let dynamic = &self.ctx.fonts.label.dynamic;
        let tempo = &self.ctx.fonts.label.tempo;
        let label_height = tempo.line_height().max(dynamic.line_height());
        let hairpin_height = dynamic.size() * DYNAMIC_HAIRPIN_HEIGHT;

        label_height.max(hairpin_height) + dynamic.size() * DYNAMIC_VOICE_GAP
    }

    fn plan_global(&mut self) {
        let spans = &self.ctx.system.spans;

        let mut indices = (0..spans.len())
            .filter(|&index| spans[index].voice.is_none())
            .collect::<Vec<_>>();

        indices.sort_by(|&lhs, &rhs| span_order(&spans[lhs], &spans[rhs]));

        self.global_lanes = indices.len();
        self.assign_lanes(&indices);
    }

    fn plan_local(&mut self) {
        let geometry = SystemGeometry::new(*self.ctx);
        let style = &self.ctx.options.elements.dynamic;
        let spans = &self.ctx.system.spans;

        let voices = self
            .ctx
            .system
            .voices
            .iter()
            .map(|voice| voice.id)
            .collect::<Vec<_>>();

        for voice_id in voices {
            if self.markers.find_dynamics(voice_id).is_some() {
                continue;
            }

            let mut candidates = (0..spans.len())
                .filter(|&index| spans[index].voice.is_some_and(|item| item.id == voice_id))
                .collect::<Vec<_>>();

            candidates.sort_by(|&lhs, &rhs| span_order(&spans[lhs], &spans[rhs]));

            let mut local = Vec::new();

            for index in candidates {
                let span = &spans[index];
                let start_x = geometry.note_x(span.start, span.start_unit) + style.left();
                let end_x = geometry.note_x(span.end, span.end_unit) - style.right();
                let span_rank = span.kind.rank();
                let hits_marker = self.markers.markers.iter().any(|marker| {
                    let (left, right) = marker.horizontal_bounds();

                    marker.kind.rank() < span_rank && start_x < right && end_x > left
                });

                if hits_marker {
                    self.top_lanes += 1;
                } else {
                    local.push(index);
                }
            }

            if !local.is_empty() {
                self.local_lanes.insert(voice_id, local.len());
                self.assign_lanes(&local);
            }
        }
    }

    fn assign_lanes(&mut self, spans: &[usize]) {
        for (lane, &index) in spans.iter().enumerate() {
            self.lanes.insert(index, lane);
        }
    }
}

fn span_order(lhs: &SystemSpan, rhs: &SystemSpan) -> std::cmp::Ordering {
    (!lhs.kind.is_tempo(), lhs.start, lhs.start_unit)
        .partial_cmp(&(!rhs.kind.is_tempo(), rhs.start, rhs.start_unit))
        .unwrap_or(std::cmp::Ordering::Equal)
}

pub fn layout_spans(
    ctx: &LayoutContext,
    layout: &MarkerLayout,
    rows: &[SystemRow],
    plan: &SpanPlan,
) -> Vec<PlacedSpan> {
    if ctx.system.spans.is_empty() {
        return Vec::new();
    }

    let style = &ctx.options.elements.dynamic;
    let geometry = SystemGeometry::new(*ctx);
    let markers = &layout.markers;
    let row_height = plan.row_height();
    let label_gap = ctx.fonts.label.dynamic.size() * DYNAMIC_SPAN_LABEL_GAP;

    let mut spans = Vec::new();

    for (span_index, span) in ctx.system.spans.iter().enumerate() {
        let center_y = if let Some(voice) = span.voice {
            let Some(row) = voice_row(rows, voice.id) else {
                continue;
            };

            if let Some(marker) = layout.find_dynamics(voice.id) {
                ctx.fonts.label.dynamic.line_center(marker.y)
            } else {
                let lane = plan.lane(span_index);
                let top = row.y - plan.local_height(voice.id);

                top + row_height * (lane as f32 + 0.5)
            }
        } else {
            let marker = layout
                .find_endpoint(span, false)
                .or_else(|| layout.find_endpoint(span, true));

            if let Some(marker) = marker {
                ctx.fonts.label.dynamic.line_center(marker.y)
            } else {
                let lane = plan.lane(span_index);

                row_height * (lane as f32 + 0.5)
            }
        };

        let label_font = if span.kind.is_tempo() {
            &ctx.fonts.label.tempo
        } else {
            &ctx.fonts.label.dynamic
        };

        let baseline = center_y + (label_font.ascent() - label_font.descent()) / 2.0;
        let fallback_start_x = geometry.note_x(span.start, span.start_unit) + style.left();
        let fallback_end_x = geometry.note_x(span.end, span.end_unit) - style.right();

        let start_x = geometry
            .note_bounds(span.start, span.start_unit, span.voice)
            .map(|(start, _)| start + style.left())
            .unwrap_or(fallback_start_x);

        let end_x = geometry
            .note_bounds(span.end, span.end_unit, span.voice)
            .map(|(_, end)| end - style.right())
            .unwrap_or(fallback_end_x);

        let start_x = match layout.find_endpoint(span, false) {
            Some(marker) => {
                let (_, right) = marker.horizontal_bounds();
                (right + label_gap).max(start_x)
            }
            None => start_x,
        };

        let end_x = match layout.find_endpoint(span, true) {
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

    resolve_span_collisions(&mut spans, markers, ctx, plan.row_height());

    spans
}

fn resolve_span_collisions(
    spans: &mut [PlacedSpan],
    markers: &[PlacedMarker],
    ctx: &LayoutContext,
    row_height: f32,
) {
    let half_height = row_height / 2.0;
    let gap = ctx.fonts.label.dynamic.size() * DYNAMIC_VOICE_GAP;

    let marker_obstacles = markers
        .iter()
        .map(|marker| (marker, marker.rect(ctx.fonts)))
        .collect::<Vec<_>>();
    let mut span_obstacles = Vec::new();

    let mut order = (0..spans.len()).collect::<Vec<_>>();

    order.sort_by_key(|&index| spans[index].kind.is_tempo()); // tempo on top of dynamics

    for index in order {
        let endpoint = &ctx.system.spans[index];
        let span = &mut spans[index];

        loop {
            let mut rect = span.rect(half_height);

            let marker_hit = marker_obstacles.iter().find(|(marker, obstacle)| {
                marker.pulse != endpoint.start
                    && marker.pulse != endpoint.end
                    && collides_with_marker(&rect, obstacle, span.kind, marker.kind, gap)
            });

            let span_hit = span_obstacles
                .iter()
                .find(|obstacle| rect.intersects(obstacle));

            let Some(obstacle) = marker_hit.map(|(_, rect)| rect).or(span_hit) else {
                break;
            };

            let shift = rect.shift_above(obstacle, gap);

            span.center_y -= shift;
            span.baseline -= shift;
        }

        span_obstacles.push(span.rect(half_height));
    }
}

fn collides_with_marker(
    span: &Rect,
    marker: &Rect,
    span_kind: SpanKind,
    marker_kind: MarkerKind,
    gap: f32,
) -> bool {
    if !span.horizontal_intersects(marker) {
        return false;
    }

    if span_kind.rank() > marker_kind.rank() {
        span.bottom > marker.top - gap
    } else {
        span.intersects(marker)
    }
}

impl PlacedSpan {
    fn rect(&self, half_height: f32) -> Rect {
        Rect {
            left: self.start_x.min(self.end_x),
            right: self.start_x.max(self.end_x),
            top: self.center_y - half_height,
            bottom: self.center_y + half_height,
        }
    }
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

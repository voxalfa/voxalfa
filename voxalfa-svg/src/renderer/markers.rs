use voxalfa_core::{ast::directives::ElementStyle, data_types::Touch};

use crate::{
    fonts::{FontMetrics, Fonts},
    model::{MarkerGlyph, MarkerKind, SystemMarker, SystemSpan, TextAnchor, VoiceInstance},
    options::RenderOptions,
    renderer::systems::LayoutContext,
};

use super::{
    collision::Rect,
    constants::*,
    geometry::{FLOAT_ERROR, SystemGeometry, segno_width},
};

pub fn layout_markers(ctx: &LayoutContext) -> MarkerLayout {
    let global_voice = ctx.system.voices.first().map(|voice| voice.id);

    let (voice_dynamics, stacked): (Vec<_>, Vec<_>) =
        ctx.system.markers.iter().partition(|marker| {
            marker.kind == MarkerKind::Dynamic
                && marker
                    .voice
                    .is_some_and(|voice| Some(voice.id) != global_voice)
        });

    let geometry = SystemGeometry::new(ctx.system, ctx.fonts, ctx.lead);

    let markers = stacked
        .into_iter()
        .map(|marker| place_marker(ctx, &geometry, marker))
        .collect::<Vec<_>>();

    let mut layout = stack_markers(ctx, markers);

    layout.markers.extend(
        voice_dynamics
            .into_iter()
            .map(|marker| place_marker(ctx, &geometry, marker)),
    );

    layout
}

fn touch_width(glyph: &MarkerGlyph, fonts: &Fonts) -> f32 {
    let size = fonts.touch.size() * TOUCH_SYMBOL_SCALE;

    match glyph {
        MarkerGlyph::Touch(Touch::Fermata) => size * FERMATA_WIDTH_SCALE,
        MarkerGlyph::Touch(Touch::Staccato) => size * STACCATO_WIDTH_SCALE,
        MarkerGlyph::Touch(Touch::Marcato) => size * MARCATO_WIDTH_SCALE,
        _ => 0.0,
    }
}

fn marker_font(kind: MarkerKind, fonts: &Fonts) -> &FontMetrics {
    match kind {
        MarkerKind::Key => &fonts.label.key,
        MarkerKind::Jump => &fonts.label.jump,
        MarkerKind::Tempo => &fonts.label.tempo,
        MarkerKind::Bpm => &fonts.label.bpm,
        MarkerKind::TimeSignature => &fonts.label.time,
        MarkerKind::Touch => &fonts.touch,
        MarkerKind::Mark => &fonts.label.mark,
        MarkerKind::Dynamic => &fonts.label.dynamic,
    }
}

fn spread_markers(markers: &mut [PlacedMarker], gap: f32) {
    markers.sort_by(|lhs, rhs| lhs.x.total_cmp(&rhs.x));

    let mut right_edge = f32::NEG_INFINITY;

    for marker in markers {
        let (left, _) = marker.horizontal_bounds();
        let shift = (right_edge + gap - left).max(0.0);

        marker.x += shift;
        right_edge = left + marker.width + shift;
    }
}

fn marker_style(kind: MarkerKind, options: &RenderOptions) -> Option<&ElementStyle> {
    match kind {
        MarkerKind::Key => Some(&options.elements.key_label),
        MarkerKind::Jump => Some(&options.elements.jump),
        MarkerKind::Tempo => Some(&options.elements.tempo),
        MarkerKind::Bpm => Some(&options.elements.bpm),
        MarkerKind::Mark => Some(&options.elements.mark),
        MarkerKind::Dynamic => Some(&options.elements.dynamic),
        MarkerKind::TimeSignature => Some(&options.elements.time_label),
        MarkerKind::Touch => None,
    }
}

fn place_marker(
    ctx: &LayoutContext,
    geometry: &SystemGeometry,
    marker: &SystemMarker,
) -> PlacedMarker {
    let font = marker_font(marker.kind, ctx.fonts);

    let width = match &marker.glyph {
        MarkerGlyph::Text(text) => font.width(text),
        MarkerGlyph::Segno => segno_width(ctx.fonts),
        MarkerGlyph::Touch(_) => touch_width(&marker.glyph, ctx.fonts),
    };

    let fallback_x = geometry.note_x(marker.pulse, marker.unit);

    let x = match marker.kind {
        MarkerKind::Dynamic | MarkerKind::Touch => geometry
            .note_center(marker.pulse, marker.unit, None)
            .unwrap_or(fallback_x),
        MarkerKind::Mark if matches!(&marker.glyph, MarkerGlyph::Segno) => geometry
            .note_center(marker.pulse, marker.unit, None)
            .unwrap_or(fallback_x),
        _ => fallback_x,
    };

    let anchor = match marker.kind {
        MarkerKind::TimeSignature => TextAnchor::Middle,
        MarkerKind::Key | MarkerKind::Tempo | MarkerKind::Bpm => TextAnchor::Start,
        MarkerKind::Dynamic | MarkerKind::Touch => TextAnchor::Middle,
        MarkerKind::Jump => TextAnchor::End,
        MarkerKind::Mark if marker.unit >= 0.5 => TextAnchor::End,
        MarkerKind::Mark => TextAnchor::Middle,
    };

    let x = marker_style(marker.kind, ctx.options)
        .map(|style| anchor.align_x(x, style))
        .unwrap_or(x);

    PlacedMarker {
        kind: marker.kind,
        glyph: marker.glyph.clone(),
        width,
        x,
        y: 0.0,
        anchor,
        pulse: marker.pulse,
        unit: marker.unit,
        voice: marker.voice,
    }
}

fn stack_markers(ctx: &LayoutContext, mut markers: Vec<PlacedMarker>) -> MarkerLayout {
    let mut order = (0..markers.len()).collect::<Vec<_>>();

    order.sort_by(|&lhs, &rhs| {
        let lhs = &markers[lhs];
        let rhs = &markers[rhs];

        lhs.kind
            .rank()
            .cmp(&rhs.kind.rank())
            .then(lhs.x.total_cmp(&rhs.x))
            .then(lhs.pulse.cmp(&rhs.pulse))
            .then(lhs.unit.total_cmp(&rhs.unit))
    });

    let mut obstacles = Vec::new();

    for index in order {
        let marker = &mut markers[index];
        let font = marker_font(marker.kind, ctx.fonts);
        let gap = font.size() * MARKER_BOTTOM_GAP;
        let mut rect = marker.rect(ctx.fonts);

        while let Some(obstacle) = obstacles.iter().find(|item| rect.intersects(item)) {
            let shift = rect.shift_above(obstacle, gap);
            marker.y -= shift;
            rect = marker.rect(ctx.fonts);
        }

        obstacles.push(rect);
    }

    let min_top = markers
        .iter()
        .map(|marker| marker.vertical_bounds(ctx.fonts).0)
        .fold(f32::INFINITY, f32::min);

    if min_top.is_finite() && min_top < 0.0 {
        for marker in &mut markers {
            marker.y -= min_top;
        }
    }

    spread_rows(ctx, &mut markers);

    let max_bottom = markers
        .iter()
        .map(|marker| marker.vertical_bounds(ctx.fonts).1)
        .fold(0.0, f32::max);

    let height = max_bottom + ctx.fonts.label.key.size() * MARKER_BOTTOM_GAP;

    MarkerLayout { markers, height }
}

fn spread_rows(ctx: &LayoutContext, markers: &mut [PlacedMarker]) {
    markers.sort_by(|lhs, rhs| {
        lhs.pulse
            .cmp(&rhs.pulse)
            .then(lhs.unit.total_cmp(&rhs.unit))
            .then(lhs.y.total_cmp(&rhs.y))
            .then(lhs.x.total_cmp(&rhs.x))
    });

    let mut start = 0;

    while start < markers.len() {
        let mut end = start + 1;

        while end < markers.len()
            && markers[start].same_note(&markers[end])
            && (markers[start].y - markers[end].y).abs() < FLOAT_ERROR
        {
            end += 1;
        }

        let gap = markers[start..end]
            .iter()
            .map(|marker| marker_font(marker.kind, ctx.fonts).size())
            .fold(0.0, f32::max)
            * MARKER_HORIZONTAL_GAP;

        spread_markers(&mut markers[start..end], gap);

        start = end;
    }
}

#[derive(Debug, Clone)]
pub struct PlacedMarker {
    pub kind: MarkerKind,
    pub glyph: MarkerGlyph,
    pub width: f32,
    pub anchor: TextAnchor,
    pub x: f32,
    pub y: f32,
    pub pulse: usize,
    pub unit: f32,
    pub voice: Option<VoiceInstance>,
}

impl PlacedMarker {
    fn same_note(&self, other: &Self) -> bool {
        self.pulse == other.pulse && (self.unit - other.unit).abs() < FLOAT_ERROR
    }

    pub fn horizontal_bounds(&self) -> (f32, f32) {
        match self.anchor {
            TextAnchor::Start => (self.x, self.x + self.width),
            TextAnchor::Middle => (self.x - self.width / 2.0, self.x + self.width / 2.0),
            TextAnchor::End => (self.x - self.width, self.x),
        }
    }

    pub fn vertical_bounds(&self, fonts: &Fonts) -> (f32, f32) {
        let font = marker_font(self.kind, fonts);
        (self.y - font.ascent(), self.y + font.descent())
    }

    pub fn rect(&self, fonts: &Fonts) -> Rect {
        let (left, right) = self.horizontal_bounds();
        let (top, bottom) = self.vertical_bounds(fonts);

        Rect {
            left,
            right,
            top,
            bottom,
        }
    }
}

#[derive(Debug)]
pub struct MarkerLayout {
    pub markers: Vec<PlacedMarker>,
    pub height: f32,
}

impl MarkerLayout {
    pub fn find_dynamics(&self, voice_id: usize) -> Option<&PlacedMarker> {
        self.markers.iter().find(|marker| {
            marker.kind == MarkerKind::Dynamic && marker.voice.map(|item| item.id) == Some(voice_id)
        })
    }

    pub fn find_endpoint(&self, span: &SystemSpan, at_end: bool) -> Option<&PlacedMarker> {
        let pulse = if at_end { span.end } else { span.start };

        self.markers.iter().find(|marker| {
            marker.voice.map(|voice| voice.id) == span.voice.map(|voice| voice.id)
                && marker.pulse == pulse
        })
    }

    pub fn shift(&mut self, offset: f32) {
        for marker in &mut self.markers {
            marker.y += offset;
        }

        self.height += offset;
    }

    pub fn ensure_height(&mut self, height: f32) {
        self.height = self.height.max(height);
    }

    pub fn bottom_baseline(&self) -> Option<f32> {
        self.markers.iter().map(|marker| marker.y).reduce(f32::max)
    }
}

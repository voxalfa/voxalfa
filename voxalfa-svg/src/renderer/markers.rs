use crate::{
    fonts::{FontMetrics, Fonts},
    layout::{MARCATO_WIDTH_SCALE, MARKER_HORIZONTAL_GAP, TOUCH_SYMBOL_SCALE},
    model::{MarkerGlyph, MarkerKind},
};

use super::{layout::PlacedMarker, primitives::TextAnchor};

pub fn marker_priority(kind: MarkerKind) -> usize {
    match kind {
        MarkerKind::Key => 0,
        MarkerKind::TimeSignature => 1,
        MarkerKind::Tempo | MarkerKind::TempoNumber => 2,
        MarkerKind::Jump => 3,
        MarkerKind::Mark => 4,
        MarkerKind::Dynamic => 5,
        MarkerKind::Touch => 6,
    }
}

pub fn touch_width(glyph: &MarkerGlyph, fonts: &Fonts) -> f32 {
    let size = fonts.marker.size() * TOUCH_SYMBOL_SCALE;

    match glyph {
        MarkerGlyph::Fermata => size * 1.7,
        MarkerGlyph::Staccato => size * 0.32,
        MarkerGlyph::Marcato => size * MARCATO_WIDTH_SCALE,
        _ => 0.0,
    }
}

pub fn marker_font(kind: MarkerKind, fonts: &Fonts) -> &FontMetrics {
    match kind {
        MarkerKind::Key | MarkerKind::Jump | MarkerKind::TempoNumber => &fonts.marker_bold,
        MarkerKind::TimeSignature => &fonts.marker_sans,
        MarkerKind::Touch => &fonts.marker,
        MarkerKind::Tempo | MarkerKind::Mark => &fonts.mark,
        MarkerKind::Dynamic => &fonts.dynamic,
    }
}

pub fn spread_markers(markers: &mut [PlacedMarker]) {
    markers.sort_by(|lhs, rhs| lhs.x.total_cmp(&rhs.x));

    let mut right_edge = f32::NEG_INFINITY;

    for marker in markers {
        let left = match marker.anchor {
            TextAnchor::Start => marker.x,
            TextAnchor::Middle => marker.x - marker.width / 2.0,
            TextAnchor::End => marker.x - marker.width,
        };

        let shift = (right_edge + MARKER_HORIZONTAL_GAP - left).max(0.0);

        marker.x += shift;
        right_edge = left + marker.width + shift;
    }
}

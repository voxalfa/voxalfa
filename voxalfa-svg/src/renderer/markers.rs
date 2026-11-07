use crate::{
    fonts::{FontMetrics, Fonts},
    model::{MarkerGlyph, MarkerKind},
    renderer::constants::{FERMATA_WIDTH_SCALE, STACCATO_WIDTH_SCALE},
};

use super::{
    constants::{MARCATO_WIDTH_SCALE, TOUCH_SYMBOL_SCALE},
    layout::PlacedMarker,
    primitives::TextAnchor,
};

pub fn marker_priority(kind: MarkerKind) -> usize {
    match kind {
        MarkerKind::Key => 0,
        MarkerKind::TimeSignature => 1,
        MarkerKind::Tempo | MarkerKind::Bpm => 2,
        MarkerKind::Jump => 3,
        MarkerKind::Mark => 4,
        MarkerKind::Dynamic => 5,
        MarkerKind::Touch => 6,
    }
}

pub fn touch_width(glyph: &MarkerGlyph, fonts: &Fonts) -> f32 {
    let size = fonts.touch.size() * TOUCH_SYMBOL_SCALE;

    match glyph {
        MarkerGlyph::Fermata => size * FERMATA_WIDTH_SCALE,
        MarkerGlyph::Staccato => size * STACCATO_WIDTH_SCALE,
        MarkerGlyph::Marcato => size * MARCATO_WIDTH_SCALE,
        _ => 0.0,
    }
}

pub fn marker_font(kind: MarkerKind, fonts: &Fonts) -> &FontMetrics {
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

pub fn spread_markers(markers: &mut [PlacedMarker], gap: f32) {
    markers.sort_by(|lhs, rhs| lhs.x.total_cmp(&rhs.x));

    let mut right_edge = f32::NEG_INFINITY;

    for marker in markers {
        let left = match marker.anchor {
            TextAnchor::Start => marker.x,
            TextAnchor::Middle => marker.x - marker.width / 2.0,
            TextAnchor::End => marker.x - marker.width,
        };

        let shift = (right_edge + gap - left).max(0.0);

        marker.x += shift;
        right_edge = left + marker.width + shift;
    }
}

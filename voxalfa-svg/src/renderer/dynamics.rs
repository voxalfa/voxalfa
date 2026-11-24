use crate::{
    model::{MarkerKind, VoiceInstance},
    renderer::systems::LayoutContext,
};

use super::{
    constants::*,
    markers::{MarkerLayout, PlacedMarker},
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

        if global_voice == Some(voice.id) {
            continue;
        }

        let Some(row) = voice_row(rows, voice.id) else {
            continue;
        };

        marker.y = row.y - offset;
    }
}

pub fn voice_row(rows: &[SystemRow], id: usize) -> Option<&SystemRow> {
    rows.iter()
        .find(|row| matches!(row.kind, SystemRowKind::Voice(current) if current.id == id))
}

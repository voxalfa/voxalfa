use voxalfa_core::{
    data_types::Voice,
    ir::solfa::{NoteKind, PulseIr},
};

use crate::{
    fonts::{FontMetrics, Fonts},
    model::{System, SystemPulse},
    symbols::{SEGNO_ASPECT_RATIO, SEGNO_HEIGHT_SCALE},
};

use super::constants::*;

pub fn find_voice<'a>(pulse: &SystemPulse<'a>, voice: Voice) -> Option<&'a PulseIr> {
    pulse.subsections.iter().find_map(|sub| {
        sub.voices
            .iter()
            .find(|(current, _)| *current == voice)
            .map(|(_, voice_pulse)| *voice_pulse)
    })
}

pub fn system_width(system: &System) -> f32 {
    let leading_width = system
        .pulses
        .first()
        .map(|pulse| pulse.pulse_width)
        .unwrap_or(0.0)
        * system.pulse_offset as f32;

    let pulses_width = system
        .pulses
        .iter()
        .map(|pulse| pulse.pulse_width)
        .sum::<f32>();

    leading_width + pulses_width
}

pub fn pulse_offsets(system: &System) -> Vec<f32> {
    let mut result = Vec::with_capacity(system.pulses.len());
    let mut x = 0.0;

    for pulse in &system.pulses {
        result.push(x);
        x += pulse.pulse_width;
    }

    result
}

fn octave_width(fonts: &Fonts) -> f32 {
    ["1", "2", "3", "4", "5", "6", "7"]
        .into_iter()
        .map(|digit| fonts.text.octave.width(digit))
        .fold(0.0, f32::max)
        + fonts.text.note.size() * GRID_RIGHT_PADDING
}

pub fn segno_width(fonts: &Fonts) -> f32 {
    fonts.label.mark.line_height() * SEGNO_HEIGHT_SCALE * SEGNO_ASPECT_RATIO
}

pub fn pulse_usable_width(pulse: &SystemPulse, fonts: &Fonts, lead: f32) -> f32 {
    let mut usable = (pulse.pulse_width - octave_width(fonts)).max(0.0);

    for sub in &pulse.subsections {
        for (_, voice_pulse) in &sub.voices {
            let total = voice_pulse
                .columns
                .iter()
                .map(|column| column.duration as u32)
                .sum::<u32>()
                .max(1);
            let mut cursor = 0u32;

            for column in &voice_pulse.columns {
                let start = cursor as f32 / total as f32;

                if start > 0.0 {
                    let extent = note_extent(&column.note, fonts, lead);
                    let available = pulse.pulse_width
                        - extent
                        - fonts.text.note.size() * NOTE_BARLINE_CLEARANCE;

                    if available <= 0.0 {
                        usable = 0.0;
                    } else {
                        usable = usable.min(available / start);
                    }
                }

                cursor += column.duration as u32;
            }
        }
    }

    usable.max(0.0)
}

pub fn prolonged_leading(fonts: &Fonts) -> f32 {
    fonts
        .text
        .note
        .width(" ")
        .max(fonts.text.note.size() * PROLONGED_LEADING_MIN)
}

fn note_extent(note: &NoteKind, fonts: &Fonts, lead: f32) -> f32 {
    match note {
        NoteKind::Note(_) => {
            let (text, octave) = note_text(note);
            let base = fonts.text.note.width(&text);
            let octave = octave
                .map(|octave| {
                    fonts.text.octave.width(&octave.abs().to_string()) / 2.0
                        + fonts.text.note.size() * OCTAVE_GAP
                })
                .unwrap_or(0.0);

            lead + base + octave
        }
        NoteKind::ProlongedNote => lead + prolonged_leading(fonts) + fonts.text.note.width("-"),
        NoteKind::EmptyNote => lead,
    }
}

pub fn baseline(row_y: f32, row_height: f32, font: &FontMetrics) -> f32 {
    let content = font.ascent() + font.descent();
    row_y + (row_height - content) / 2.0 + font.ascent()
}

pub fn note_text(note: &NoteKind) -> (String, Option<i8>) {
    match note {
        NoteKind::Note(note) => (
            note.text(),
            if note.octave == 0 {
                None
            } else {
                Some(note.octave)
            },
        ),
        NoteKind::ProlongedNote => ("-".to_string(), None),
        NoteKind::EmptyNote => (String::new(), None),
    }
}

pub fn division_symbol(cursor: u32, total: u32) -> &'static str {
    if total == 0 {
        return "";
    }

    let double = cursor.saturating_mul(2);
    let quad = cursor.saturating_mul(4);

    if double == total {
        "."
    } else if quad == total {
        ","
    } else if quad == total.saturating_mul(3) {
        ".,"
    } else if double.is_multiple_of(total) {
        "."
    } else if quad.is_multiple_of(total) {
        ","
    } else {
        ""
    }
}

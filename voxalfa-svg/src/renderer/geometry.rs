use voxalfa_core::{
    ast::directives::ElementStyle,
    ir::solfa::{NoteKind, PulseIr},
};

use crate::{
    fonts::Fonts,
    model::{System, SystemPulse, VoiceInstance},
    renderer::text::note_text,
    symbols::{SEGNO_ASPECT_RATIO, SEGNO_HEIGHT_SCALE},
};

use super::{constants::*, context::LayoutContext};

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

pub const FLOAT_ERROR: f32 = 0.001;

fn find_voice<'a>(pulse: &SystemPulse<'a>, voice: VoiceInstance) -> Option<&'a PulseIr> {
    pulse.stack.iter().find_map(|sub| {
        sub.voices
            .iter()
            .find(|(current, _)| current.id == voice.id)
            .map(|(_, voice_pulse)| *voice_pulse)
    })
}

pub struct SystemGeometry<'a, 'b> {
    system: &'a System<'b>,
    fonts: &'a Fonts,
    lead: f32,
    usable_widths: Vec<f32>,
    note: &'a ElementStyle,
}

impl<'a, 'b> SystemGeometry<'a, 'b> {
    pub fn new(ctx: LayoutContext<'a, 'b>) -> Self {
        let note = &ctx.options.elements.note;
        let usable_widths = ctx
            .system
            .pulses
            .iter()
            .map(|pulse| Self::compute_usable_width(pulse, ctx.fonts, ctx.lead, note))
            .collect();

        Self {
            system: ctx.system,
            fonts: ctx.fonts,
            lead: ctx.lead,
            usable_widths,
            note,
        }
    }

    pub fn usable_width(&self, pulse_id: usize) -> f32 {
        self.usable_widths[pulse_id]
    }

    pub fn note_x(&self, pulse_id: usize, unit: f32) -> f32 {
        self.system.pulse_offsets[pulse_id] + unit * self.usable_widths[pulse_id] + self.lead
    }

    pub fn note_center(
        &self,
        pulse_id: usize,
        unit: f32,
        voice: Option<VoiceInstance>,
    ) -> Option<f32> {
        self.note_bounds(pulse_id, unit, voice)
            .map(|(start, end)| (start + end) / 2.0)
    }

    pub fn note_bounds(
        &self,
        pulse_id: usize,
        unit: f32,
        voice: Option<VoiceInstance>,
    ) -> Option<(f32, f32)> {
        let pulse = self.system.pulses.get(pulse_id)?;
        let usable_width = self.usable_widths[pulse_id];

        let voices = match voice {
            Some(voice) => vec![voice],
            None => self.system.voices.clone(),
        };

        for voice in voices {
            let Some(voice_pulse) = find_voice(pulse, voice) else {
                continue;
            };

            let total = voice_pulse.factor as f32;
            let mut cursor = 0;

            for (index, column) in voice_pulse.columns.iter().enumerate() {
                let start = cursor as f32 / total;

                cursor += column.duration as u32;

                let end = cursor as f32 / total;
                let is_last = index + 1 == voice_pulse.columns.len();

                if unit + FLOAT_ERROR >= start
                    && (unit < end - FLOAT_ERROR || (is_last && unit <= end + FLOAT_ERROR))
                {
                    if matches!(column.note, NoteKind::EmptyNote) {
                        continue;
                    }

                    let (text, _) = note_text(&column.note);
                    let glyph_width = self.fonts.text.note.width(&text);
                    let leading = match column.note {
                        NoteKind::ProlongedNote => self.lead + prolonged_leading(self.fonts),
                        _ => self.lead,
                    };

                    let start_x = self.system.pulse_offsets[pulse_id]
                        + start * usable_width
                        + leading
                        + self.note.left();

                    return Some((start_x, start_x + glyph_width + self.note.right()));
                }
            }
        }

        None
    }

    fn compute_usable_width(
        pulse: &SystemPulse,
        fonts: &Fonts,
        lead: f32,
        note: &ElementStyle,
    ) -> f32 {
        let mut usable = (pulse.pulse_width - octave_width(fonts)).max(0.0);

        for sub in &pulse.stack {
            for (_, voice_pulse) in &sub.voices {
                let total = voice_pulse.factor as f32;
                let mut cursor = 0u32;

                for column in &voice_pulse.columns {
                    let start = cursor as f32 / total;

                    if start > 0.0 {
                        let extent = note_extent(&column.note, fonts, lead);

                        let available = pulse.pulse_width
                            - extent
                            - note.x_space()
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

        usable
    }
}

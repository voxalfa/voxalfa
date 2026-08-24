use crate::{
    emitter::SvgEmitter,
    error::{Error, Result},
    fonts::FontInterface,
    layout::{
        MARGIN_X, MARGIN_Y, PRINTABLE_WIDTH, SYSTEM_GAP, UNDERLINE_Y_OFFSET, VOICE_LINE_HEIGHT,
    },
    types::{BarlineElement, Element, LineSystem, TextElement, UnderlineElement},
    visitor::SvgVisitor,
};

use voxalfa_core::{
    ast::solfa::PulseAccent,
    data_types::TimeSignature,
    ir::{
        SubSectionIr,
        solfa::{NoteKind, PulseIr},
    },
    output::{
        FinalOutput,
        lyrics::{LyricsBuilder, LyricsMap},
        voice::VoiceSet,
    },
};

pub struct Renderer<'a> {
    data: FinalOutput,
    time: TimeSignature,
    font: FontInterface<'a>,
    col_width: f32,
    col_factor: u8,
    lyrics_map: LyricsMap<f32>,
}

impl<'a> Renderer<'a> {
    pub fn new(data: FinalOutput) -> Result<Self> {
        let font = FontInterface::new()?;
        let max_factor = data.resolve_maximum_factor();
        let time = data.header.get_params(|p| &p.time);
        let builder = LyricsBuilder::new(&font);
        let (col_width, lyrics_map) = builder.build_map::<SvgVisitor>(&data, max_factor);

        if let Some(&time) = time {
            Ok(Self {
                data,
                time,
                font,
                col_width,
                lyrics_map,
                col_factor: max_factor,
            })
        } else {
            Err(Error::MissingHeaderField("time"))
        }
    }

    pub fn render_to_svg(self) -> String {
        let elements = self.render();
        SvgEmitter::render_to_svg(&elements)
    }

    fn render(&self) -> Vec<Element> {
        let mut current_y = MARGIN_Y + 40.0;
        let mut elements = Vec::new();

        for system in self.collect_systems() {
            current_y = self.render_system(&system, current_y, &mut elements);
        }

        elements
    }

    fn render_system(
        &self,
        system: &LineSystem<'_>,
        start_y: f32,
        elements: &mut Vec<Element>,
    ) -> f32 {
        let max_line_pulses = self.calculate_line_pulses(system);
        let default_pulse_step = self.col_factor as f32 * self.col_width;
        let voices_height = self.calculate_voices_height(&system.voices);

        let line_content_width = max_line_pulses as f32 * default_pulse_step;
        let total_empty_space = (PRINTABLE_WIDTH - line_content_width).max(0.0);
        let extra_space_per_pulse = total_empty_space / max_line_pulses as f32;
        let justified_pulse_step = default_pulse_step + extra_space_per_pulse;

        let mut current_x = MARGIN_X;
        let mut current_y = start_y;
        let mut line_pulse_count = 0;

        for section in system.internals {
            for sub_section in &section.items {
                let max_pulses = sub_section
                    .solfa
                    .iter()
                    .map(|s| s.pulses.len())
                    .max()
                    .unwrap_or(0);

                for pulse_index in 0..max_pulses {
                    // Line wrapping: advance current_y by the vertical height of a single wrapped line
                    if line_pulse_count >= max_line_pulses {
                        current_x = MARGIN_X;
                        current_y += voices_height + SYSTEM_GAP;
                        line_pulse_count = 0;
                    }

                    self.render_pulse_slot(
                        sub_section,
                        pulse_index,
                        current_x,
                        current_y,
                        voices_height,
                        justified_pulse_step,
                        elements,
                    );

                    current_x += justified_pulse_step;
                    line_pulse_count += 1;
                }
            }
        }

        // Return the bottom position for the next system block
        current_y + voices_height + SYSTEM_GAP
    }

    fn render_pulse_slot(
        &self,
        sub_section: &SubSectionIr,
        pulse_index: usize,
        x: f32,
        y: f32,
        voices_height: f32,
        justified_pulse_step: f32,
        elements: &mut Vec<Element>,
    ) {
        for (voice_idx, solfa) in sub_section.solfa.iter().enumerate() {
            let Some(pulse) = solfa.pulses.get(pulse_index) else {
                continue;
            };

            if pulse.accent == PulseAccent::Strong {
                elements.push(Element::Barline(BarlineElement {
                    x,
                    y1: y - self.font.solfa.ascent,
                    y2: y + voices_height + self.font.solfa.descent,
                }));
            }

            let voice_y = y + (voice_idx as f32 * VOICE_LINE_HEIGHT);

            self.render_voice_pulse(pulse, x, voice_y, justified_pulse_step, elements);
        }
    }

    fn render_voice_pulse(
        &self,
        pulse: &PulseIr,
        x: f32,
        voice_y: f32,
        justified_pulse_step: f32,
        elements: &mut Vec<Element>,
    ) {
        let accent_str = self.format_pulse_accent(pulse.accent);

        if pulse.expanded {
            elements.push(Element::Text(TextElement {
                x,
                y: voice_y,
                content: accent_str.to_string(),
                class: "solfa",
            }));

            return;
        }

        let mut col_x = x;
        let mut clock = 0;

        for (step, column) in pulse.columns.iter().enumerate() {
            let lead = match (step, clock, pulse.factor) {
                (0, _, _) => accent_str,
                (1, 1, 2) | (_, 2, 4) => ".",
                (1, 3, 4) => ".,",
                (_, 1, 4) | (_, 3, 4) => ",",
                _ if step == clock => ",",
                _ => "",
            };

            self.create_note_element(lead, &column.note, col_x, voice_y, elements);

            let column_ratio = column.duration as f32 / pulse.factor as f32;
            let scaled_width = justified_pulse_step * column_ratio;

            if column.underline.left || column.underline.right {
                elements.push(Element::Underline(UnderlineElement {
                    x1: col_x,
                    x2: col_x + scaled_width,
                    y: voice_y + UNDERLINE_Y_OFFSET,
                }));
            }

            col_x += scaled_width;
            clock += column.duration as usize;
        }
    }

    fn create_note_element(
        &self,
        lead: &str,
        note: &NoteKind,
        x: f32,
        y: f32,
        elements: &mut Vec<Element>,
    ) {
        let mut content = lead.to_string();
        let lead_width = self.font.solfa.get_width(lead);

        let x_pos = if lead == ".," {
            x - lead_width
        } else {
            x + lead_width
        };

        let (note_text, _octave) = match note {
            NoteKind::Note(note) => (Some(note.text()), Some(note.octave)),
            NoteKind::ProlongedNote => (Some("-".to_string()), None),
            NoteKind::EmptyNote => (None, None),
        };

        if let Some(note) = note_text {
            content.push_str(&note);
        }

        elements.push(Element::Text(TextElement {
            x: x_pos,
            y,
            content,
            class: "solfa",
        }));

        // TODO: Octave marker
        // if let Some(octave) = octave.filter(|&o| o != 0) {
        //     let marker_y = match octave > 0 {
        //         true => y - self.font.solfa.ascent + 8.0,
        //         false => y + self.font.solfa.descent,
        //     };
        //
        //     elements.push(Element::Text(TextElement {
        //         x: x + 8.,
        //         y: marker_y,
        //         content: octave.abs().to_string(),
        //         class: "octave",
        //     }));
        // }
    }

    fn format_pulse_accent(&self, accent: PulseAccent) -> &'static str {
        match accent {
            PulseAccent::Strong => " ",
            PulseAccent::Medium => "|",
            PulseAccent::Weak => ":",
        }
    }

    fn calculate_voices_height(&self, voices: &[VoiceSet]) -> f32 {
        if voices.is_empty() {
            return 0.0;
        }

        let mut total_height = 0.0;

        for voice_set in voices {
            if voice_set.is_empty() {
                continue;
            }

            let voice_count = voice_set.len();

            if voice_count > 0 {
                total_height += (voice_count - 1) as f32 * VOICE_LINE_HEIGHT;
            }
        }

        total_height
    }

    fn calculate_line_pulses(&self, system: &LineSystem<'_>) -> usize {
        let pulse_width = self.col_factor as f32 * self.col_width;
        if pulse_width <= 0.0 || system.time.top == 0 {
            return 1;
        }

        let max_raw_pulses = (PRINTABLE_WIDTH / pulse_width).floor() as usize;
        let n = max_raw_pulses / system.time.top as usize;
        let measure_count = usize::max(1, n);

        measure_count * system.time.top as usize
    }

    fn collect_systems(&self) -> Vec<LineSystem<'_>> {
        let sections = self.data.body.sections.as_slice();

        if sections.is_empty() {
            return Vec::new();
        }

        let mut result = Vec::new();
        let mut current_time = self.time;
        let mut current_voices = sections[0].voice_sets();
        let mut chunk_start = 0;

        for (index, section) in sections.iter().enumerate() {
            let new_time = section.params.time.as_ref().map(|t| t.value);
            let new_voices = section.voice_sets();

            let time_changed = new_time.map_or(false, |t| t != current_time);
            let voices_changed = new_voices != current_voices;

            if (time_changed || voices_changed) && index > chunk_start {
                result.push(LineSystem {
                    time: current_time,
                    voices: current_voices,
                    internals: &sections[chunk_start..index],
                });

                chunk_start = index;
                current_voices = new_voices;

                if let Some(t) = new_time {
                    current_time = t;
                }
            }
        }

        if chunk_start < sections.len() {
            result.push(LineSystem {
                time: current_time,
                voices: current_voices,
                internals: &sections[chunk_start..],
            });
        }

        result
    }
}

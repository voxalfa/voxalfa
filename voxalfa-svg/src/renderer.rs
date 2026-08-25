use crate::{
    emitter::SvgEmitter,
    error::{Error, Result},
    fonts::FontInterface,
    layout::{
        A4_HEIGHT_PX, A4_PADDING, A4_WIDTH_PX, PRINTABLE_WIDTH, SYSTEM_GAP, UNDERLINE_Y_OFFSET,
        VOICE_LINE_HEIGHT,
    },
    types::{Element, LineSystem, TextElement},
    visitor::SvgVisitor,
};

use taffy::{
    AlignItems, Display, FlexDirection, JustifyContent, NodeId, Rect, Size, Style, TaffyTree,
    style_helpers::{TaffyMaxContent, auto, fr, length, line, percent},
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
                lyrics_map,
                col_width,
                col_factor: max_factor,
            })
        } else {
            Err(Error::MissingHeaderField("time"))
        }
    }

    pub fn render_to_svg(self) -> Result<String> {
        let mut tree = TaffyTree::<()>::new();

        let elements = self.render(&mut tree)?;
        let emitter = SvgEmitter::new(tree);
        let svg = emitter.render_to_svg(&elements)?;

        Ok(svg)
    }

    fn render(&self, tree: &mut TaffyTree<()>) -> Result<Vec<Element>> {
        let mut elements = Vec::new();

        let document_node = tree.new_leaf(Style {
            size: Size {
                width: length(A4_WIDTH_PX),
                height: length(A4_HEIGHT_PX),
            },
            padding: Rect::length(A4_PADDING),
            flex_direction: FlexDirection::Column,
            gap: Size {
                width: length(0.0),
                height: length(SYSTEM_GAP),
            },
            ..Default::default()
        })?;

        for system in self.collect_systems() {
            let system_node_id = self.render_system(&system, tree, &mut elements)?;
            tree.add_child(document_node, system_node_id)?;
        }

        tree.compute_layout(document_node, Size::MAX_CONTENT)?;

        Ok(elements)
    }

    fn render_system(
        &self,
        system: &LineSystem<'_>,
        tree: &mut TaffyTree<()>,
        elements: &mut Vec<Element>,
    ) -> Result<NodeId> {
        let max_line_pulses = self.calculate_line_pulses(system);

        let system_node_id = tree.new_leaf(Style {
            flex_direction: FlexDirection::Column,
            ..Default::default()
        })?;

        Ok(system_node_id)
    }

    fn render_pulse_slot(
        &self,
        sub_section: &SubSectionIr,
        pulse_index: usize,
        tree: &mut TaffyTree,
        elements: &mut Vec<Element>,
    ) -> Result<NodeId> {
        let pulse_node_id = tree.new_leaf(Style {
            size: Size {
                width: length(self.col_width * self.col_factor as f32),
                height: percent(100),
            },
            ..Default::default()
        })?;

        for (voice_idx, solfa) in sub_section.solfa.iter().enumerate() {
            let Some(pulse) = solfa.pulses.get(pulse_index) else {
                continue;
            };

            if pulse.accent == PulseAccent::Strong {
                // elements.push(Element::Barline(BarlineElement {
                //     x,
                //     y1: y - self.font.solfa.ascent,
                //     y2: y + voices_height + self.font.solfa.descent,
                // }));
            }

            // self.render_voice_pulse(pulse, tree, elements);
        }

        Ok(pulse_node_id)
    }

    // fn render_voice_pulse(
    //     &self,
    //     pulse: &PulseIr,
    //     tree: &mut TaffyTree
    //     elements: &mut Vec<Element>,
    // ) {
    //     let accent_str = self.format_pulse_accent(pulse.accent);
    //
    //     if pulse.expanded {
    //         elements.push(ElementKind::Text(TextElement {
    //             x,
    //             y: voice_y,
    //             content: accent_str.to_string(),
    //             class: "solfa",
    //         }));
    //
    //         return;
    //     }
    //
    //     let mut col_x = x;
    //     let mut clock = 0;
    //
    //     for (step, column) in pulse.columns.iter().enumerate() {
    //         let lead = match (step, clock, pulse.factor) {
    //             (0, _, _) => accent_str,
    //             (1, 1, 2) | (_, 2, 4) => ".",
    //             (1, 3, 4) => ".,",
    //             (_, 1, 4) | (_, 3, 4) => ",",
    //             _ if step == clock => ",",
    //             _ => "",
    //         };
    //
    //         self.create_note_element(lead, &column.note, col_x, voice_y, elements);
    //
    //         let column_ratio = column.duration as f32 / pulse.factor as f32;
    //         let scaled_width = justified_pulse_step * column_ratio;
    //
    //         if column.underline.left || column.underline.right {
    //             elements.push(ElementKind::Underline(UnderlineElement {
    //                 x1: col_x,
    //                 x2: col_x + scaled_width,
    //                 y: voice_y + UNDERLINE_Y_OFFSET,
    //             }));
    //         }
    //
    //         col_x += scaled_width;
    //         clock += column.duration as usize;
    //     }
    // }
    //
    // fn create_note_element(
    //     &self,
    //     lead: &str,
    //     note: &NoteKind,
    //     x: f32,
    //     y: f32,
    //     elements: &mut Vec<ElementKind>,
    // ) {
    //     let mut content = lead.to_string();
    //     let lead_width = self.font.solfa.get_width(lead);
    //
    //     let x_pos = if lead == ".," {
    //         x - lead_width
    //     } else {
    //         x + lead_width
    //     };
    //
    //     let (note_text, _octave) = match note {
    //         NoteKind::Note(note) => (Some(note.text()), Some(note.octave)),
    //         NoteKind::ProlongedNote => (Some("-".to_string()), None),
    //         NoteKind::EmptyNote => (None, None),
    //     };
    //
    //     if let Some(note) = note_text {
    //         content.push_str(&note);
    //     }
    //
    //     elements.push(ElementKind::Text(TextElement {
    //         x: x_pos,
    //         y,
    //         content,
    //         class: "solfa",
    //     }));
    //
    //     // TODO: Octave marker
    //     // if let Some(octave) = octave.filter(|&o| o != 0) {
    //     //     let marker_y = match octave > 0 {
    //     //         true => y - self.font.solfa.ascent + 8.0,
    //     //         false => y + self.font.solfa.descent,
    //     //     };
    //     //
    //     //     elements.push(Element::Text(TextElement {
    //     //         x: x + 8.,
    //     //         y: marker_y,
    //     //         content: octave.abs().to_string(),
    //     //         class: "octave",
    //     //     }));
    //     // }
    // }

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

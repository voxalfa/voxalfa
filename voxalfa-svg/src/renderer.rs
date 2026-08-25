use crate::{
    emitter::SvgEmitter,
    error::{Error, Result},
    fonts::FontInterface,
    layout::{A4_HEIGHT_PX, A4_PADDING, A4_WIDTH_PX, LINE_GAP, PRINTABLE_WIDTH},
    types::{Element, ElementKind, LineSystem, RenderContext, TextElement, Underline},
    visitor::SvgVisitor,
};

use taffy::{
    Display, FlexDirection, NodeId, Rect, Size, Style, TaffyTree,
    prelude::{fr, minmax, span},
    style_helpers::{auto, length, percent},
};
use voxalfa_core::{
    ast::solfa::PulseAccent,
    data_types::TimeSignature,
    ir::solfa::{NoteKind, PulseIr},
    output::{
        FinalOutput,
        lyrics::{LyricsBuilder, LyricsMap},
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
        let mut context = RenderContext::new(tree);

        let body_node = self.render_body(&mut context)?;
        let elements = context.into_elements();

        let document_node = tree.new_with_children(
            Style {
                size: Size {
                    width: length(A4_WIDTH_PX),
                    height: length(A4_HEIGHT_PX),
                },
                padding: Rect::length(A4_PADDING),
                flex_direction: FlexDirection::Column,
                ..Default::default()
            },
            &[body_node],
        )?;

        tree.compute_layout(
            document_node,
            Size {
                width: length(A4_WIDTH_PX),
                height: length(A4_HEIGHT_PX),
            },
        )?;

        Ok(elements)
    }

    fn render_body(&self, ctx: &mut RenderContext) -> Result<NodeId> {
        let body_node = ctx.tree.new_with_children(
            Style {
                size: Size {
                    width: percent(100),
                    height: percent(100),
                },
                flex_direction: FlexDirection::Column,
                ..Default::default()
            },
            &[],
        )?;

        for system in self.collect_systems() {
            let system_node = self.render_system(&system, ctx)?;
            ctx.tree.add_child(body_node, system_node)?;
        }

        Ok(body_node)
    }

    fn render_system(&self, system: &LineSystem<'_>, ctx: &mut RenderContext) -> Result<NodeId> {
        let system_node = ctx.tree.new_with_children(
            Style {
                size: Size {
                    width: percent(100),
                    height: auto(),
                },
                flex_direction: FlexDirection::Column,
                gap: length(LINE_GAP),
                ..Default::default()
            },
            &[],
        )?;

        let max_line_pulses = self.calculate_line_pulses(system);
        let total_subdivisions = max_line_pulses * self.col_factor as usize;
        let lead_col_width = self.font.solfa.get_width(".,");

        let mut grid_columns = Vec::with_capacity(total_subdivisions * 2);

        for _ in 0..total_subdivisions {
            grid_columns.push(length(lead_col_width));
            grid_columns.push(minmax(length(self.col_width), fr(1.0)));
        }

        let line_style = Style {
            display: Display::Grid,
            grid_template_columns: grid_columns,
            size: Size {
                width: length(PRINTABLE_WIDTH),
                height: auto(),
            },
            ..Default::default()
        };

        let mut section_id = 0;
        let mut sub_section_id = 0;
        let mut voice_id = 0;
        let mut pulse_id = 0;

        let mut end_section_id = 0;
        let mut end_sub_section_id = 0;
        let mut end_pulse_id = 0;

        let mut line_node = ctx.tree.new_with_children(line_style.clone(), &[])?;

        loop {
            let mut pulses_collected = 0;

            let start_section_id = section_id;
            let start_sub_section_id = sub_section_id;
            let start_pulse_id = pulse_id;

            while pulses_collected < max_line_pulses {
                let Some(section) = system.internals.get(section_id) else {
                    break;
                };

                let Some(sub_section) = section.items.get(sub_section_id) else {
                    section_id += 1;
                    sub_section_id = 0;
                    pulse_id = 0;
                    continue;
                };

                let Some(line) = sub_section.solfa.get(voice_id) else {
                    sub_section_id += 1;
                    pulse_id = 0;

                    if sub_section_id >= section.items.len() {
                        section_id += 1;
                        sub_section_id = 0;
                    }
                    continue;
                };

                let needed = max_line_pulses - pulses_collected;
                let available = line.pulses.len().saturating_sub(pulse_id);
                let count = available.min(needed);

                for pulse in &line.pulses[pulse_id..pulse_id + count] {
                    for node in self.render_pulse_columns(pulse, ctx)? {
                        ctx.tree.add_child(line_node, node)?;
                    }
                }

                pulses_collected += count;
                pulse_id += count;

                if pulse_id >= line.pulses.len() {
                    section_id += 1;
                    pulse_id = 0;
                }

                if voice_id == 0 {
                    end_section_id = section_id;
                    end_sub_section_id = sub_section_id;
                    end_pulse_id = pulse_id;
                }
            }

            if pulses_collected == 0 && voice_id == 0 {
                break;
            }

            ctx.tree.add_child(system_node, line_node)?;
            line_node = ctx.tree.new_with_children(line_style.clone(), &[])?;
            voice_id += 1;

            let voice_exists = system
                .internals
                .get(start_section_id)
                .and_then(|sec| sec.items.get(start_sub_section_id))
                .map_or(false, |sub| voice_id < sub.solfa.len());

            if voice_exists {
                section_id = start_section_id;
                sub_section_id = start_sub_section_id;
                pulse_id = start_pulse_id;
            } else {
                voice_id = 0;
                section_id = end_section_id;
                sub_section_id = end_sub_section_id;
                pulse_id = end_pulse_id;
            }
        }

        Ok(system_node)
    }

    fn render_pulse_columns(
        &self,
        pulse: &PulseIr,
        ctx: &mut RenderContext,
    ) -> Result<Vec<NodeId>> {
        let mut result = Vec::new();
        let accent_str = self.format_pulse_accent(pulse.accent);

        if pulse.expanded {
            let track_span = self.col_factor as u16 * 2; // spans across lead

            let node = ctx.tree.new_leaf(Style {
                grid_column: span(track_span),
                size: Size {
                    width: percent(100),
                    height: length(self.font.solfa.ascent),
                },
                ..Default::default()
            })?;

            ctx.add_element(
                node,
                ElementKind::Text(TextElement {
                    content: accent_str.to_string(),
                    class: "solfa",
                    underline: Underline::None,
                }),
            );

            result.push(node);
        } else {
            let mut clock = 0;

            for (step, column) in pulse.columns.iter().enumerate() {
                let lead_symbol =
                    self.resolve_column_prefix(pulse.accent, step, clock, pulse.factor);

                let lead_node = ctx.tree.new_leaf(Style {
                    size: Size {
                        width: percent(100),
                        height: length(self.font.solfa.ascent),
                    },
                    ..Default::default()
                })?;

                if !lead_symbol.is_empty() {
                    ctx.add_element(
                        lead_node,
                        ElementKind::Text(TextElement {
                            content: lead_symbol.to_string(),
                            class: "solfa",
                            underline: Underline::None,
                        }),
                    );
                }
                result.push(lead_node);

                let base_span =
                    (column.duration as u16 * self.col_factor as u16) / pulse.factor as u16;
                let total_grid_span = base_span + (base_span - 1);

                let note_node = ctx.tree.new_leaf(Style {
                    grid_column: span(total_grid_span),
                    size: Size {
                        width: auto(),
                        height: length(self.font.solfa.ascent),
                    },
                    ..Default::default()
                })?;

                ctx.add_element(
                    note_node,
                    ElementKind::Text(TextElement {
                        content: self.format_note(&column.note),
                        class: "solfa",
                        underline: Underline::None,
                    }),
                );
                result.push(note_node);

                clock += column.duration as usize;
            }
        }

        Ok(result)
    }

    fn resolve_column_prefix(
        &self,
        accent: PulseAccent,
        step: usize,
        clock: usize,
        factor: u8,
    ) -> &'static str {
        match (step, clock, factor) {
            (0, _, _) => self.format_pulse_accent(accent),
            (1, 1, 2) | (_, 2, 4) => ".",
            (1, 3, 4) => ".,",
            (_, 1, 4) | (_, 3, 4) => ",",
            _ if step == clock => ",",
            _ => "",
        }
    }

    fn format_note(&self, note: &NoteKind) -> String {
        match note {
            NoteKind::Note(note) => note.text(),
            NoteKind::ProlongedNote => "-".to_string(),
            NoteKind::EmptyNote => String::new(),
        }
    }

    fn format_pulse_accent(&self, accent: PulseAccent) -> &'static str {
        match accent {
            PulseAccent::Strong => " ",
            PulseAccent::Medium => "|",
            PulseAccent::Weak => ":",
        }
    }

    fn calculate_line_pulses(&self, system: &LineSystem<'_>) -> usize {
        let lead_col_width = self.font.solfa.get_width(".,");
        let single_subdivision_width = self.col_width + lead_col_width;
        let full_pulse_width = self.col_factor as f32 * single_subdivision_width;

        if full_pulse_width <= 0.0 || system.time.top == 0 {
            return 1;
        }

        let max_raw_pulses = (PRINTABLE_WIDTH / full_pulse_width).floor() as usize;
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

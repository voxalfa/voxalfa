use crate::{
    emitter::SvgEmitter,
    error::{Error, Result},
    fonts::FontInterface,
    layout::{A4_HEIGHT_PX, A4_PADDING, A4_WIDTH_PX, LINE_GAP, PRINTABLE_WIDTH, SYSTEM_GAP},
    types::{Element, ElementKind, LineSystem, RenderContext, TextElement, UnderlineElement},
    visitor::SvgVisitor,
};

use taffy::{
    Display, FlexDirection, NodeId, Rect, Size, Style, TaffyTree,
    prelude::{fr, minmax, span, zero},
    style_helpers::{auto, length, percent},
};
use voxalfa_core::{
    ast::solfa::PulseAccent,
    data_types::TimeSignature,
    ir::solfa::{NoteKind, PulseColumn, PulseIr},
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
                col_width: col_width.max(20.),
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
                gap: length(SYSTEM_GAP),
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

        let grid_style = Style {
            display: Display::Grid,
            grid_template_columns: grid_columns,
            size: Size {
                width: length(PRINTABLE_WIDTH),
                height: auto(),
            },
            gap: Size {
                width: zero(),
                height: length(LINE_GAP),
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

        let grid_node = ctx.tree.new_with_children(grid_style, &[])?;

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
                let voice_count = sub_section.solfa.len();

                for pulse in &line.pulses[pulse_id..pulse_id + count] {
                    let columns = self.render_pulse_columns(pulse, voice_id, voice_count, ctx)?;

                    for node in columns {
                        ctx.tree.add_child(grid_node, node)?;
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

            let remainder = max_line_pulses - pulses_collected;

            if remainder > 0 {
                let extra_node = ctx.tree.new_leaf(Style {
                    grid_column: span(remainder as u16 * self.col_factor as u16 * 2),
                    ..Default::default()
                })?;

                ctx.tree.add_child(grid_node, extra_node)?;
            }

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

        ctx.tree.add_child(system_node, grid_node)?;

        Ok(system_node)
    }

    fn render_pulse_columns(
        &self,
        pulse: &PulseIr,
        voice_id: usize,
        voice_count: usize,
        ctx: &mut RenderContext,
    ) -> Result<Vec<NodeId>> {
        let mut result = Vec::new();
        let is_strong = pulse.accent == PulseAccent::Strong;

        if voice_id == 0 && is_strong {
            let barline_node = ctx.tree.new_leaf(Style {
                grid_column: span(1),
                grid_row: span(voice_count as u16),
                size: Size {
                    width: auto(),
                    height: auto(),
                },
                ..Default::default()
            })?;

            ctx.add_element(barline_node, ElementKind::Barline);
            result.push(barline_node);
        }

        let mut clock = 0;

        if pulse.expanded {
            let col_span = self.col_factor as u16 * 2 - (is_strong as u16);
            let extra_node = ctx.tree.new_leaf(Style {
                grid_column: span(col_span),
                ..Default::default()
            })?;

            result.push(extra_node);

            return Ok(result);
        }

        for (step, column) in pulse.columns.iter().enumerate() {
            let lead_symbol = self.resolve_column_prefix(pulse, step, clock);

            if pulse.accent != PulseAccent::Strong || step != 0 {
                let lead_node = self.render_lead_node(lead_symbol, ctx)?;
                result.push(lead_node);
            }

            let (note_node, note_width) = self.render_note_node(column, pulse.factor, ctx)?;
            result.push(note_node);

            if column.underline.left {
                ctx.underline_node = Some(note_node);
            }

            if let Some(node_id) = ctx.underline_node.take_if(|_| column.underline.right) {
                ctx.add_element(
                    node_id,
                    ElementKind::Underline(UnderlineElement {
                        end_node: note_node,
                        real_width: note_width,
                    }),
                );
            }

            clock += column.duration as usize;
        }

        Ok(result)
    }

    fn render_lead_node(&self, lead_symbol: &str, ctx: &mut RenderContext) -> Result<NodeId> {
        let lead_node = ctx.tree.new_leaf(Style {
            size: Size {
                width: auto(),
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
                }),
            );
        }

        Ok(lead_node)
    }

    fn render_note_node(
        &self,
        column: &PulseColumn,
        pulse_factor: u8,
        ctx: &mut RenderContext,
    ) -> Result<(NodeId, f32)> {
        let base_span = (column.duration as u16 * self.col_factor as u16) / pulse_factor as u16;
        let total_grid_span = base_span + (base_span - 1);
        let text = self.format_note(&column.note);
        let width = self.font.solfa.get_width(&text);

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
                content: text,
                class: "solfa",
            }),
        );

        Ok((note_node, width))
    }

    fn resolve_column_prefix(&self, pulse: &PulseIr, step: usize, clock: usize) -> &'static str {
        match (step, clock, pulse.factor) {
            (0, _, _) => self.format_pulse_accent(pulse.accent),
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

use voxalfa_core::{
    ast::{lyrics::LyricOperatorKind, solfa::PulseAccent},
    data_types::TimeSignature,
    ir::{SubSectionIr, solfa::NoteKind},
    output::{
        FinalOutput,
        lyrics::{LyricsBuilder, LyricsMap},
    },
};

use crate::{
    error::{Error, Result},
    fonts::FontInterface,
    layout::PRINTABLE_WIDTH,
    types::{CellContent, GridCell, LineSystem, LyricChunk, VerseState, VirtualGrid},
};

pub struct GridResolver<'a> {
    pub data: FinalOutput,
    pub time: TimeSignature,
    pub font: FontInterface<'a>,
    pub verses: usize,
    pub col_width: f32,
    pub col_factor: u8,
    pub lyrics_map: LyricsMap<f32>,
}

impl<'a> GridResolver<'a> {
    pub fn new(data: FinalOutput) -> Result<Self> {
        let font = FontInterface::new()?;
        let max_factor = data.resolve_maximum_factor();
        let time = data.header.get_params(|p| &p.time);
        let verses = data
            .header
            .get_metadata(|p| &p.verses)
            .copied()
            .unwrap_or_default();
        let builder = LyricsBuilder::new(&font);
        let (col_width, lyrics_map) =
            builder.build_map::<crate::visitor::SvgVisitor>(&data, max_factor);

        if let Some(&time) = time {
            Ok(Self {
                data,
                time,
                font,
                verses,
                lyrics_map,
                col_width: col_width.max(20.),
                col_factor: max_factor,
            })
        } else {
            Err(Error::MissingHeaderField("time"))
        }
    }

    pub fn collect_systems(&self) -> Vec<LineSystem<'_>> {
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

    pub fn build_system_grid(&self, system: &LineSystem<'_>) -> Result<VirtualGrid> {
        let max_line_pulses = self.calculate_line_pulses(system);
        let total_subdivisions = max_line_pulses * self.col_factor as usize;

        let mut grid = VirtualGrid::new(total_subdivisions * 2);

        let mut section_id = 0;
        let mut sub_section_id = 0;
        let mut voice_id = 0;
        let mut pulse_id = 0;

        let mut end_section_id = 0;
        let mut end_sub_section_id = 0;
        let mut end_pulse_id = 0;

        let mut verse_states = vec![VerseState::default(); self.verses];

        loop {
            let mut pulses_collected = 0;
            let mut row: Vec<GridCell> = Vec::new();

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
                    let cells = self.resolve_pulse_columns(pulse, voice_id, voice_count);
                    row.extend(cells);
                }

                if voice_id == 0 {
                    self.update_verses_state(
                        sub_section,
                        pulse_id,
                        max_line_pulses,
                        &mut verse_states,
                    );
                }

                pulses_collected += count;
                pulse_id += count;

                if pulse_id >= line.pulses.len() {
                    section_id += 1;
                    sub_section_id = 0;
                    pulse_id = 0;

                    for state in &mut verse_states {
                        state.lyric_id = 0;
                    }
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
                row.push(GridCell::new(
                    CellContent::Empty,
                    remainder as u16 * self.col_factor as u16 * 2,
                ));
            }

            grid.add_row(row);

            voice_id += 1;

            let voice_exists = system
                .internals
                .get(start_section_id)
                .and_then(|sec| sec.items.get(start_sub_section_id))
                .map_or(false, |sub| voice_id < sub.solfa.len());

            if !voice_exists {
                self.resolve_lyrics(&mut grid, &mut verse_states)?;
            }

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

        Ok(grid)
    }

    fn resolve_pulse_columns(
        &self,
        pulse: &voxalfa_core::ir::solfa::PulseIr,
        voice_id: usize,
        voice_count: usize,
    ) -> Vec<GridCell> {
        let mut result = Vec::new();
        let is_strong = pulse.accent == PulseAccent::Strong;

        if voice_id == 0 && is_strong {
            result.push(GridCell {
                content: CellContent::Barline,
                col_span: 1,
                row_span: voice_count as u16,
            });
        }

        let mut clock = 0;

        if pulse.expanded {
            let col_span = self.col_factor as u16 * 2 - (is_strong as u16);
            result.push(GridCell::new(CellContent::Empty, col_span));
            return result;
        }

        for (step, column) in pulse.columns.iter().enumerate() {
            let lead_symbol = self.resolve_column_prefix(pulse, step, clock);

            if pulse.accent != PulseAccent::Strong || step != 0 {
                result.push(GridCell::new(
                    CellContent::Text {
                        content: lead_symbol.to_string(),
                        class: "solfa",
                    },
                    1,
                ));
            }

            let base_span = (column.duration as u16 * self.col_factor as u16) / pulse.factor as u16;
            let total_grid_span = base_span + (base_span - 1);
            let text = self.format_note(&column.note);

            result.push(GridCell::new(
                CellContent::Text {
                    content: text,
                    class: "solfa",
                },
                total_grid_span,
            ));

            clock += column.duration as usize;
        }

        result
    }

    fn resolve_lyrics(
        &self,
        grid: &mut VirtualGrid,
        verse_states: &mut Vec<VerseState>,
    ) -> Result<()> {
        for verse in verse_states {
            if verse.line.is_empty() {
                continue;
            }

            let mut row: Vec<GridCell> = Vec::new();

            for chunk in verse.line.drain(..) {
                let (content, span_value) = match chunk {
                    LyricChunk::String { content, span } => (content, span),
                    LyricChunk::Opertator(LyricOperatorKind::Concat) => ("-".to_string(), 1_usize),
                    _ => (String::new(), 1_usize),
                };

                row.push(GridCell::new(
                    CellContent::Text {
                        content,
                        class: "lyric",
                    },
                    span_value as u16,
                ));
            }

            grid.add_row(row);
        }

        Ok(())
    }

    fn update_verses_state(
        &self,
        sub_section: &SubSectionIr,
        pulse_id: usize,
        pulse_count: usize,
        state: &mut Vec<VerseState>,
    ) {
        let needed = pulse_count * self.col_factor as usize;

        for verse_id in 0..self.verses {
            let verse = &sub_section.lyrics[verse_id];
            let verse_state = &mut state[verse_id];

            let mut view_id = pulse_id;
            let mut collected = 0;
            let mut view_offset = 0;

            while collected < needed {
                let Some(current) = verse.columns.get(verse_state.lyric_id) else {
                    break;
                };

                let operator = verse.operators.get(verse_state.lyric_id);

                let mut remainder = current.span;
                let mut span_value = 0;

                let is_strong_accent = sub_section
                    .solfa
                    .first()
                    .is_some_and(|s| s.pulses[view_id].accent == PulseAccent::Strong);

                if view_offset == 0 && is_strong_accent {
                    verse_state.line.push(LyricChunk::Placeholder);
                }

                while remainder != 0 {
                    let view = &sub_section.views[view_id];

                    let view_spans = view
                        .durations
                        .iter()
                        .map(|d| (d * 2 * self.col_factor) / view.factor)
                        .collect::<Vec<_>>();

                    span_value += view_spans[view_offset] as usize;
                    view_offset += 1;
                    remainder -= 1;

                    if view_offset >= view_spans.len() {
                        view_id += 1;
                        view_offset = 0;
                    }
                }

                let entry = &self.lyrics_map[&current.sid];

                collected += span_value as usize;
                verse_state.lyric_id += 1;

                verse_state.line.push(LyricChunk::String {
                    content: entry.content.clone(),
                    span: span_value - 1,
                });

                if let Some(op) = operator
                    && collected < needed
                {
                    verse_state.line.push(LyricChunk::Opertator(op.value));
                }
            }
        }
    }

    fn resolve_column_prefix(
        &self,
        pulse: &voxalfa_core::ir::solfa::PulseIr,
        step: usize,
        clock: usize,
    ) -> &'static str {
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
}

use voxalfa_core::{
    ast::solfa::PulseAccent,
    data_types::Voice,
    ir::solfa::{NoteKind, PulseIr},
};

use crate::{
    error::Result,
    fonts::Fonts,
    model::{
        DynamicSpanKind, MarkerGlyph, MarkerKind, ResolvedScore, System, SystemLyricToken,
        SystemPulse,
    },
    options::RenderOptions,
    renderer::text::{division_symbol, note_text},
};

use super::{
    constants::*,
    geometry::{baseline, prolonged_leading, pulse_offsets, pulse_usable_width},
    layout::{SystemLayout, SystemRowKind},
    primitives::{BraceSide, emit_brace, emit_line, emit_segno, emit_text, emit_touch},
};

fn find_voice<'a>(pulse: &SystemPulse<'a>, voice: Voice) -> Option<&'a PulseIr> {
    pulse.stack.iter().find_map(|sub| {
        sub.voices
            .iter()
            .find(|(current, _)| *current == voice)
            .map(|(_, voice_pulse)| *voice_pulse)
    })
}

pub fn emit_system(
    svg: &mut String,
    fonts: &Fonts,
    score: &ResolvedScore,
    placement: SystemPlacement<'_>,
    options: &RenderOptions,
) -> Result<()> {
    let pulse_width = placement
        .system
        .pulses
        .first()
        .map(|pulse| pulse.pulse_width)
        .unwrap_or(0.0);

    let origin_x = options.page_margin.left
        + placement.brace_padding_left
        + placement.body_offset
        + placement.system.pulse_offset as f32 * pulse_width;

    let mut render = SystemRender::new(svg, fonts, placement, score.lead_width, origin_x, options);

    render.emit()
}

pub struct SystemPlacement<'a> {
    pub system: &'a System<'a>,
    pub layout: &'a SystemLayout,
    pub body_offset: f32,
    pub brace_padding_left: f32,
    pub y: f32,
    pub is_last_system: bool,
}

struct SystemRender<'a> {
    svg: &'a mut String,
    fonts: &'a Fonts,
    system: &'a System<'a>,
    layout: &'a SystemLayout,
    xs: Vec<f32>,
    usable_widths: Vec<f32>,
    origin_x: f32,
    lead: f32,
    y: f32,
    is_last_system: bool,
    options: &'a RenderOptions,
}

impl<'a> SystemRender<'a> {
    fn new(
        svg: &'a mut String,
        fonts: &'a Fonts,
        placement: SystemPlacement<'a>,
        lead: f32,
        origin_x: f32,
        options: &'a RenderOptions,
    ) -> Self {
        let xs = pulse_offsets(placement.system);

        let usable_widths = placement
            .system
            .pulses
            .iter()
            .map(|pulse| pulse_usable_width(pulse, fonts, lead))
            .collect();

        Self {
            svg,
            fonts,
            system: placement.system,
            layout: placement.layout,
            xs,
            usable_widths,
            origin_x,
            lead,
            y: placement.y,
            is_last_system: placement.is_last_system,
            options,
        }
    }

    fn emit(&mut self) -> Result<()> {
        let system = self.system;
        let layout = self.layout;
        let y = self.y;

        for volta in &layout.voltas {
            let start_x = self.origin_x + volta.start_x;
            let end_x = self.origin_x + volta.end_x;
            let line_y = y + volta.line_y;

            emit_line(self.svg, start_x, line_y, end_x, line_y, "volta")?;

            let tick = self.fonts.label.volta.size() * VOLTA_END_TICK;

            emit_line(self.svg, start_x, line_y, start_x, line_y + tick, "volta")?;

            if !volta.open_end {
                emit_line(self.svg, end_x, line_y, end_x, line_y + tick, "volta")?;
            }

            emit_text(
                self.svg,
                self.origin_x + volta.text_x,
                y + volta.text_y,
                &volta.text,
                "volta-label",
                "start",
            )?;
        }

        for marker in &layout.markers {
            let x = self.origin_x + marker.x;
            let baseline = y + marker.y;

            match &marker.glyph {
                MarkerGlyph::Text(text) => emit_text(
                    self.svg,
                    x,
                    baseline,
                    text,
                    match marker.kind {
                        MarkerKind::Dynamic => "dynamic-label",
                        MarkerKind::Jump => "jump-label",
                        MarkerKind::Key => "key-label",
                        MarkerKind::Tempo => "tempo-label",
                        MarkerKind::Bpm => "bpm-label",
                        MarkerKind::Mark => "mark-label",
                        MarkerKind::TimeSignature => "time-label",
                        MarkerKind::Touch => "marker",
                    },
                    marker.anchor.as_str(),
                )?,
                MarkerGlyph::Segno => emit_segno(self.svg, x, baseline, self.fonts)?,
                MarkerGlyph::Touch(_) => {
                    emit_touch(self.svg, &marker.glyph, x, baseline, self.fonts)?
                }
            }
        }

        for span in &layout.dynamic_spans {
            let start_x = self.origin_x + span.start_x;
            let end_x = self.origin_x + span.end_x;

            if span.continuation {
                emit_line(
                    self.svg,
                    start_x,
                    y + span.center_y,
                    end_x,
                    y + span.center_y,
                    "dynamic-dash",
                )?;
            } else if let Some(label) = &span.label {
                emit_text(
                    self.svg,
                    start_x,
                    y + span.baseline,
                    label,
                    "dynamic-label",
                    "start",
                )?;

                let dash_start =
                    start_x + span.label_width + self.fonts.label.dynamic.size() * DYNAMIC_DASH_GAP;

                if dash_start < end_x {
                    emit_line(
                        self.svg,
                        dash_start,
                        y + span.center_y,
                        end_x,
                        y + span.center_y,
                        "dynamic-dash",
                    )?;
                }
            } else {
                let center_y = y + span.center_y;
                let half = self.fonts.label.dynamic.size() * DYNAMIC_HAIRPIN_HEIGHT / 2.0;

                let (left_top, left_bottom, right_top, right_bottom) = match span.kind {
                    DynamicSpanKind::Crescendo => {
                        (center_y, center_y, center_y - half, center_y + half)
                    }
                    DynamicSpanKind::Decrescendo => {
                        (center_y - half, center_y + half, center_y, center_y)
                    }
                };

                emit_line(
                    self.svg,
                    start_x,
                    left_top,
                    end_x,
                    right_top,
                    "dynamic-hairpin",
                )?;
                emit_line(
                    self.svg,
                    start_x,
                    left_bottom,
                    end_x,
                    right_bottom,
                    "dynamic-hairpin",
                )?;
            }
        }

        for row in &layout.rows {
            match row.kind {
                SystemRowKind::Voice(voice) => {
                    let voice_baseline = baseline(y + row.y, row.height, &self.fonts.text.note);

                    self.emit_voice_row(voice, voice_baseline)?;
                }
                SystemRowKind::Lyric {
                    group_index,
                    verse_index,
                } => {
                    let group = &system.lyric_groups[group_index];
                    let tokens = &group.verses[verse_index];
                    let lyric_baseline = baseline(y + row.y, row.height, &self.fonts.text.lyric);

                    self.emit_lyric_row(tokens, lyric_baseline)?;
                }
            }
        }

        if let Some(first_voice) = layout
            .rows
            .iter()
            .find(|row| matches!(row.kind, SystemRowKind::Voice(_)))
        {
            let inset = self.fonts.text.note.size() * BARLINE_BLOCK_INSET;
            let top = y + first_voice.y + inset;
            let bottom = y + layout.height - inset;

            self.emit_barlines(top, bottom)?;
            self.emit_voice_braces(y + first_voice.y, y + layout.height, !self.is_last_system)?;
        }

        Ok(())
    }

    fn emit_voice_braces(&mut self, top: f32, bottom: f32, close: bool) -> Result<()> {
        let system = self.system;
        let inset = self.fonts.text.note.size() * BARLINE_BLOCK_INSET;
        let top = top + inset;
        let bottom = bottom - inset;

        if bottom <= top {
            return Ok(());
        }

        let line_left = self.origin_x;
        let line_right = self.origin_x
            + system
                .pulses
                .iter()
                .map(|pulse| pulse.pulse_width)
                .sum::<f32>();

        let size = self.fonts.text.note.size();
        let gap = size * BRACE_GAP;
        let half_width = size * BRACE_HALF_WIDTH;

        emit_brace(
            self.svg,
            line_left - gap - half_width,
            top,
            bottom,
            half_width,
            BraceSide::Left,
        )?;

        if close {
            emit_brace(
                self.svg,
                line_right + half_width,
                top,
                bottom,
                half_width,
                BraceSide::Right,
            )?;
        }

        Ok(())
    }

    fn emit_barlines(&mut self, top: f32, bottom: f32) -> Result<()> {
        let system = self.system;
        let origin_x = self.origin_x;

        for (pulse_id, pulse) in system.pulses.iter().enumerate() {
            let x = origin_x + self.xs[pulse_id];

            if pulse.measure_start {
                emit_line(self.svg, x, top, x, bottom, "barline")?;
            }

            if pulse.section_end {
                let end_x = x + pulse.pulse_width;
                let gap = self.fonts.text.note.size() * DOUBLE_BARLINE_GAP;

                emit_line(self.svg, end_x - gap, top, end_x - gap, bottom, "barline")?;
                emit_line(self.svg, end_x, top, end_x, bottom, "barline")?;
            }
        }

        Ok(())
    }

    fn emit_voice_row(&mut self, voice: Voice, baseline: f32) -> Result<()> {
        let mut underline: Option<UnderlineState> = None;
        let mut last_note_end = self.origin_x;
        let note_offset = self.options.elements.note.left();

        let underline_y = baseline
            + self.fonts.text.note.descent()
            + self.fonts.text.note.size() * SOLFA_UNDERLINE_OFFSET;
        let octave_y = underline_y
            - self.fonts.text.octave.descent()
            - self.fonts.text.octave.size() * BOTTOM_OCTAVE_UNDERLINE_GAP;

        for (pulse_id, pulse) in self.system.pulses.iter().enumerate() {
            let Some(voice_pulse) = find_voice(pulse, voice) else {
                let cell_x = self.origin_x + self.xs[pulse_id];
                self.emit_pulse_accent(pulse, cell_x, baseline)?;
                continue;
            };

            let usable_width = self
                .usable_widths
                .get(pulse_id)
                .copied()
                .unwrap_or(pulse.pulse_width);

            let total = voice_pulse
                .columns
                .iter()
                .map(|column| column.duration as u32)
                .sum::<u32>()
                .max(1);

            let mut cursor = 0u32;

            for (column_id, column) in voice_pulse.columns.iter().enumerate() {
                let duration = column.duration as u32;

                let start = if cursor == 3 && total == 4 {
                    0.70 // add more room for 1/4 cells
                } else {
                    cursor as f32 / total as f32
                };

                let cell_x = self.origin_x + self.xs[pulse_id] + start * usable_width;

                let mut note_start = cell_x + self.lead;
                let mut note_end = note_start;
                let mut symbol = "";

                if column_id == 0 {
                    self.emit_pulse_accent(pulse, cell_x, baseline)?;
                } else if !voice_pulse.expanded {
                    symbol = division_symbol(cursor, total);
                }

                if !symbol.is_empty() {
                    let style = &self.options.elements.divider;

                    emit_text(
                        self.svg,
                        cell_x + self.lead / 2.0 + (style.left() - style.right()) / 2.0,
                        baseline + style.top() - style.bottom(),
                        symbol,
                        "divider-text",
                        "middle",
                    )?;
                }

                if !matches!(column.note, NoteKind::EmptyNote) {
                    let (text, octave) = note_text(&column.note);
                    let glyph_width = self.fonts.text.note.width(&text);

                    let note_leading = match column.note {
                        NoteKind::ProlongedNote => self.lead + prolonged_leading(self.fonts),
                        _ => self.lead,
                    };

                    let glyph_start = cell_x + note_leading + note_offset;
                    let glyph_center = glyph_start + glyph_width / 2.0;

                    note_start = glyph_start;
                    note_end = glyph_start + glyph_width;
                    last_note_end = note_end;

                    emit_text(
                        self.svg,
                        glyph_center,
                        baseline,
                        &text,
                        "note-text",
                        "middle",
                    )?;

                    if let Some(octave) = octave {
                        let style = &self.options.elements.octave;

                        let octave_x = glyph_center
                            + glyph_width / 2.0
                            + self.fonts.text.note.size() * OCTAVE_GAP
                            + (style.left() - style.right()) / 2.0;

                        let octave_y = if octave > 0 {
                            baseline - self.fonts.text.note.size() * TOP_OCTAVE_OFFSET_FACTOR
                        } else {
                            octave_y
                        } + style.top()
                            - style.bottom();

                        emit_text(
                            self.svg,
                            octave_x,
                            octave_y,
                            &octave.abs().to_string(),
                            "octave-text",
                            "middle",
                        )?;
                    }
                }

                if column.underline.left {
                    underline = Some(UnderlineState { start: note_start });
                }

                if column.underline.right
                    && let Some(open) = underline.take()
                {
                    emit_line(
                        self.svg,
                        open.start,
                        underline_y,
                        note_end,
                        underline_y,
                        "underline",
                    )?;
                }

                cursor += duration;
            }
        }

        if let Some(open) = underline {
            emit_line(
                self.svg,
                open.start,
                underline_y,
                last_note_end,
                underline_y,
                "underline",
            )?;
        }

        Ok(())
    }

    fn emit_pulse_accent(&mut self, pulse: &SystemPulse, cell_x: f32, baseline: f32) -> Result<()> {
        let fonts = self.fonts;
        let lead = self.lead;

        match pulse.accent {
            PulseAccent::Strong => {}
            PulseAccent::Medium => {
                let x = cell_x + lead / 2.0;
                let note_center =
                    baseline - (fonts.text.note.ascent() - fonts.text.note.descent()) / 2.0;
                let half = fonts.text.note.size() / 2.0;

                emit_line(
                    self.svg,
                    x,
                    note_center - half,
                    x,
                    note_center + half,
                    "medium-accent",
                )?;
            }
            PulseAccent::Weak => {
                let style = &self.options.elements.divider;

                emit_text(
                    self.svg,
                    cell_x + lead / 2.0 + (style.left() - style.right()) / 2.0,
                    baseline + style.top() - style.bottom(),
                    ":",
                    "divider-text",
                    "middle",
                )?;
            }
        }

        Ok(())
    }

    fn emit_lyric_row(&mut self, tokens: &[SystemLyricToken], baseline: f32) -> Result<()> {
        let system = self.system;
        let fonts = self.fonts;
        let origin_x = self.origin_x;
        let lead = self.lead;

        for token in tokens {
            if token.text.is_empty() {
                continue;
            }

            let end_width = self
                .usable_widths
                .get(token.end)
                .copied()
                .unwrap_or(system.pulses[token.end].pulse_width);

            let end_x = origin_x + self.xs[token.end] + token.end_unit * end_width;

            let start_width = self
                .usable_widths
                .get(token.start)
                .copied()
                .unwrap_or(system.pulses[token.start].pulse_width);

            let style = &self.options.elements.lyrics;

            let start_unit = if (token.start_unit - 0.75).abs() < f32::EPSILON {
                0.70 // add more room for 1/4 cells
            } else {
                token.start_unit
            };

            let x =
                origin_x + self.xs[token.start] + start_unit * start_width + lead + style.left();

            let end_x = (end_x - style.right()).max(x);

            emit_text(self.svg, x, baseline, &token.text, "lyrics-text", "start")?;

            if token.underline {
                let min_width = fonts.text.lyric.size() * MIN_UNDERLINE_WIDTH;
                let width = fonts.text.lyric.width(&token.text).max(min_width);
                let available = (end_x - x).max(min_width);
                let width = width.min(available);

                let underline_y = baseline
                    + fonts.text.lyric.descent()
                    + fonts.text.lyric.size() * LYRIC_UNDERLINE_OFFSET;

                emit_line(
                    self.svg,
                    x,
                    underline_y,
                    x + width,
                    underline_y,
                    "underline",
                )?;
            }
        }

        Ok(())
    }
}

struct UnderlineState {
    start: f32,
}

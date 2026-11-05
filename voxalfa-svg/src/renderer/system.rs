use voxalfa_core::{
    ast::solfa::PulseAccent,
    data_types::Voice,
    ir::solfa::{NoteKind, PulseIr},
};

use crate::{
    error::Result,
    fonts::Fonts,
    layout::*,
    model::{
        DynamicSpanKind, MarkerGlyph, MarkerKind, ResolvedScore, System, SystemLyricToken,
        SystemPulse,
    },
    options::RenderOptions,
};

use super::{
    geometry::{
        baseline, division_symbol, note_text, prolonged_leading, pulse_offsets, pulse_usable_width,
    },
    layout::{SystemLayout, SystemRowKind},
    primitives::{emit_line, emit_segno, emit_text, emit_touch},
};

fn find_voice<'a>(pulse: &SystemPulse<'a>, voice: Voice) -> Option<&'a PulseIr> {
    pulse.subsections.iter().find_map(|sub| {
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

    let origin_x = options.min_margin
        + placement.body_offset
        + placement.system.pulse_offset as f32 * pulse_width;

    let mut render = SystemRender::new(svg, fonts, placement, score.lead_width, origin_x);

    render.emit()
}

pub struct SystemPlacement<'a> {
    pub system: &'a System<'a>,
    pub layout: &'a SystemLayout,
    pub body_offset: f32,
    pub y: f32,
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
}

impl<'a> SystemRender<'a> {
    fn new(
        svg: &'a mut String,
        fonts: &'a Fonts,
        placement: SystemPlacement<'a>,
        lead: f32,
        origin_x: f32,
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
        }
    }

    fn emit(&mut self) -> Result<()> {
        let system = self.system;
        let layout = self.layout;
        let y = self.y;

        let mut block_top: Option<f32> = None;

        for volta in &layout.voltas {
            let start_x = self.origin_x + volta.start_x;
            let end_x = self.origin_x + volta.end_x;
            let line_y = y + volta.line_y;

            emit_line(self.svg, start_x, line_y, end_x, line_y, "volta")?;
            emit_line(
                self.svg,
                start_x,
                line_y,
                start_x,
                line_y + VOLTA_END_TICK,
                "volta",
            )?;
            emit_line(
                self.svg,
                end_x,
                line_y,
                end_x,
                line_y + VOLTA_END_TICK,
                "volta",
            )?;
            emit_text(
                self.svg,
                self.origin_x + volta.text_x,
                y + volta.text_y,
                &volta.text,
                "volta-text",
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
                        MarkerKind::Dynamic => "dynamic",
                        MarkerKind::Jump | MarkerKind::Key | MarkerKind::TempoNumber => {
                            "marker-bold"
                        }
                        MarkerKind::Tempo | MarkerKind::Mark => "marker-italic",
                        MarkerKind::TimeSignature => "marker-sans",
                        MarkerKind::Touch => "marker",
                    },
                    marker.anchor.as_str(),
                )?,
                MarkerGlyph::Segno => emit_segno(self.svg, x, baseline, self.fonts)?,
                MarkerGlyph::Fermata | MarkerGlyph::Staccato | MarkerGlyph::Marcato => {
                    emit_touch(self.svg, &marker.glyph, x, baseline, self.fonts)?
                }
            }
        }

        for span in &layout.dynamic_spans {
            let start_x = self.origin_x + span.start_x;
            let end_x = self.origin_x + span.end_x;

            if let Some(label) = &span.label {
                emit_text(
                    self.svg,
                    start_x,
                    y + span.baseline,
                    label,
                    "dynamic",
                    "start",
                )?;

                let dash_start = start_x + span.label_width + DYNAMIC_DASH_GAP;

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
                let half = DYNAMIC_HAIRPIN_HEIGHT / 2.0;

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
                    if block_top.is_none() {
                        block_top = Some(y + row.y + BARLINE_BLOCK_INSET);
                    }

                    let voice_baseline = baseline(y + row.y, row.height, &self.fonts.solfa);

                    self.emit_voice_row(voice, voice_baseline)?;
                }
                SystemRowKind::Lyric {
                    group_index,
                    verse_index,
                } => {
                    let voice_bottom = y + row.y - BARLINE_BLOCK_INSET;

                    if let Some(top) = block_top.take() {
                        self.emit_barlines(top, voice_bottom)?;
                    }

                    let group = &system.lyric_groups[group_index];
                    let tokens = &group.verses[verse_index];
                    let lyric_baseline = baseline(y + row.y, row.height, &self.fonts.lyric);

                    self.emit_lyric_row(tokens, lyric_baseline)?;
                }
            }
        }

        if let Some(top) = block_top {
            let voice_bottom = y + layout.height - BARLINE_BLOCK_INSET;
            self.emit_barlines(top, voice_bottom)?;
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

                emit_line(
                    self.svg,
                    end_x - DOUBLE_BARLINE_GAP,
                    top,
                    end_x - DOUBLE_BARLINE_GAP,
                    bottom,
                    "barline",
                )?;

                emit_line(self.svg, end_x, top, end_x, bottom, "barline")?;
            }
        }

        Ok(())
    }

    fn emit_voice_row(&mut self, voice: Voice, baseline: f32) -> Result<()> {
        let mut underline: Option<UnderlineState> = None;
        let mut last_note_end = self.origin_x;

        let underline_y = baseline + self.fonts.solfa.descent() + SOLFA_UNDERLINE_OFFSET;
        let octave_y = underline_y - self.fonts.octave.descent() - BOTTOM_OCTAVE_UNDERLINE_GAP;

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
                let start = cursor as f32 / total as f32;
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
                    emit_text(
                        self.svg,
                        cell_x + self.lead / 2.0,
                        baseline,
                        symbol,
                        "divider",
                        "middle",
                    )?;
                }

                if !matches!(column.note, NoteKind::EmptyNote) {
                    let (text, octave) = note_text(&column.note);
                    let glyph_width = self.fonts.solfa.width(&text);

                    let note_leading = match column.note {
                        NoteKind::ProlongedNote => self.lead + prolonged_leading(self.fonts),
                        _ => self.lead,
                    };

                    let glyph_start = cell_x + note_leading;
                    let glyph_center = glyph_start + glyph_width / 2.0;

                    note_start = glyph_start;
                    note_end = glyph_start + glyph_width;
                    last_note_end = note_end;

                    emit_text(self.svg, glyph_center, baseline, &text, "solfa", "middle")?;

                    if let Some(octave) = octave {
                        let octave_x = glyph_center + glyph_width / 2.0 + OCTAVE_GAP;

                        let octave_y = if octave > 0 {
                            baseline - self.fonts.solfa.size() * TOP_OCTAVE_OFFSET_FACTOR
                        } else {
                            octave_y
                        };

                        emit_text(
                            self.svg,
                            octave_x,
                            octave_y,
                            &octave.abs().to_string(),
                            "octave",
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
                let note_center = baseline - (fonts.solfa.ascent() - fonts.solfa.descent()) / 2.0;
                let half = fonts.solfa.size() / 2.0;

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
                emit_text(
                    self.svg,
                    cell_x + lead / 2.0,
                    baseline,
                    ":",
                    "divider",
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

            let x = origin_x + self.xs[token.start] + token.start_unit * start_width + lead;
            let end_x = end_x.max(x);

            emit_text(self.svg, x, baseline, &token.text, "lyric", "start")?;

            if token.underline {
                let width = fonts.lyric.width(&token.text).max(MIN_UNDERLINE_WIDTH);
                let available = (end_x - x).max(MIN_UNDERLINE_WIDTH);
                let width = width.min(available);

                emit_line(
                    self.svg,
                    x,
                    baseline + fonts.lyric.descent() + LYRIC_UNDERLINE_OFFSET,
                    x + width,
                    baseline + fonts.lyric.descent() + LYRIC_UNDERLINE_OFFSET,
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

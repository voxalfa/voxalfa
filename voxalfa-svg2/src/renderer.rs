use std::fmt::Write;

use base64::{Engine, engine::general_purpose::STANDARD};
use voxalfa_core::{
    ast::solfa::PulseAccent,
    data_types::Voice,
    ir::solfa::{NoteKind, PulseIr},
    output::FinalOutput,
};

use crate::{
    error::Result,
    fonts::{FontMetrics, Fonts, LYRIC_FAMILY, LYRIC_FONT, SOLFA_FAMILY, SOLFA_FONT},
    model::{HeaderView, ResolvedScore, System, SystemLyricGroup, SystemLyricToken, SystemPulse},
    options::RenderOptions,
    resolver,
};

/// Horizontal gap between a note letter and its octave digit.
const OCTAVE_GAP: f32 = 2.0;

/// High level SVG renderer.
///
/// The renderer is stateless apart from its configuration. Call [`Renderer::render`]
/// with a [`FinalOutput`] to obtain one SVG document containing all pages.
pub struct Renderer {
    options: RenderOptions,
}

impl Default for Renderer {
    fn default() -> Self {
        Self::new(RenderOptions::default())
    }
}

impl Renderer {
    pub fn new(options: RenderOptions) -> Self {
        Self { options }
    }

    pub fn options(&self) -> &RenderOptions {
        &self.options
    }

    pub fn render(&self, data: &FinalOutput) -> Result<String> {
        let fonts = Fonts::new(&self.options)?;
        let score = resolver::resolve(data, &fonts, &self.options)?;
        let header = self.layout_header(&score.header, &fonts);
        let pages = self.paginate(&score, header.height);

        let page_count = pages.len().max(1);
        let total_height = self.options.page.height * page_count as f32;

        let mut svg = String::with_capacity(32 * 1024);

        writeln!(
            svg,
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {:.2} {:.2}" width="{:.2}px" height="{:.2}px">"#,
            self.options.page.width, total_height, self.options.page.width, total_height,
        )?;

        self.emit_defs(&mut svg)?;

        for (page_id, page) in pages.iter().enumerate() {
            let offset_y = page_id as f32 * self.options.page.height;

            writeln!(svg, r#"<g transform="translate(0 {offset_y:.2})">"#)?;
            writeln!(
                svg,
                r##"<rect x="0" y="0" width="{:.2}" height="{:.2}" fill="#ffffff"/>"##,
                self.options.page.width, self.options.page.height,
            )?;

            if page_id == 0 {
                self.emit_header(&mut svg, &header)?;
            }

            for placed in &page.systems {
                self.emit_system(
                    &mut svg,
                    &fonts,
                    &score,
                    &score.systems[placed.index],
                    placed.y,
                )?;
            }

            if page_count > 1 {
                writeln!(
                    svg,
                    r#"<text x="{:.2}" y="{:.2}" text-anchor="middle" class="page-number">{} / {}</text>"#,
                    self.options.page.width / 2.0,
                    self.options.page.height - self.options.margin / 2.0,
                    page_id + 1,
                    page_count,
                )?;
            }

            writeln!(svg, "</g>")?;
        }

        writeln!(svg, "</svg>")?;

        Ok(svg)
    }

    fn paginate(&self, score: &ResolvedScore, header_height: f32) -> Vec<Page> {
        let bottom = self.options.page.height - self.options.margin;
        let mut pages = vec![Page::default()];
        let mut y = self.options.margin;

        y += header_height;

        if header_height > 0.0 {
            y += self.options.header_gap;
        }

        for (index, system) in score.systems.iter().enumerate() {
            let rows = Self::system_rows(system);
            let height = Self::rows_height(&rows, &self.options);

            if y + height > bottom && !pages.last().unwrap().systems.is_empty() {
                pages.push(Page::default());
                y = self.options.margin;
            }

            pages
                .last_mut()
                .unwrap()
                .systems
                .push(PlacedSystem { index, y });

            y += height + self.options.system_gap;
        }

        pages
    }

    /// Build one row group per sub-section. A sub-section owns its voices and
    /// the lyric lines that belong to those voices, so multi-lyric (`++`)
    /// systems keep each lyric line directly below the corresponding voice
    /// group instead of collecting every lyric at the bottom of the system.
    fn system_rows<'s, 'a>(system: &'s System<'a>) -> Vec<VoiceGroupLayout<'s>> {
        let subsection_count = system
            .pulses
            .iter()
            .map(|pulse| pulse.subsections.len())
            .max()
            .unwrap_or(0);

        let mut groups = Vec::new();

        for subsection in 0..subsection_count {
            let mut voices = Vec::new();

            for pulse in &system.pulses {
                let Some(sub) = pulse.subsections.get(subsection) else {
                    continue;
                };

                for (voice, _) in &sub.voices {
                    if !voices.contains(voice) {
                        voices.push(*voice);
                    }
                }
            }

            let lyrics = system
                .lyric_groups
                .iter()
                .filter(|group| group.subsection == subsection && !group.tokens.is_empty())
                .collect::<Vec<_>>();

            if voices.is_empty() && lyrics.is_empty() {
                continue;
            }

            groups.push(VoiceGroupLayout {
                subsection,
                voices,
                lyrics,
            });
        }

        groups
    }

    fn rows_height(rows: &[VoiceGroupLayout], options: &RenderOptions) -> f32 {
        rows.iter()
            .map(|group| {
                group.voices.len() as f32 * options.voice_row_height
                    + group.lyrics.len() as f32 * options.lyric_row_height
            })
            .sum()
    }

    fn emit_system(
        &self,
        svg: &mut String,
        fonts: &Fonts,
        score: &ResolvedScore,
        system: &System,
        y: f32,
    ) -> Result<()> {
        let groups = Self::system_rows(system);
        let xs = Self::pulse_offsets(system);
        let usable_widths = system
            .pulses
            .iter()
            .map(|pulse| Self::pulse_usable_width(pulse, fonts, score.lead_width))
            .collect::<Vec<_>>();
        let pulse_width = system
            .pulses
            .first()
            .map(|pulse| pulse.pulse_width)
            .unwrap_or(0.0);
        let origin_x = self.options.margin + system.grid_offset as f32 * pulse_width;
        let mut row_y = y;

        for group in &groups {
            let voice_top = row_y + 6.0;

            for voice in &group.voices {
                let baseline = baseline(row_y, self.options.voice_row_height, &fonts.solfa);

                self.emit_voice_row(
                    svg,
                    fonts,
                    system,
                    &xs,
                    &usable_widths,
                    origin_x,
                    score.lead_width,
                    group.subsection,
                    *voice,
                    baseline,
                )?;

                row_y += self.options.voice_row_height;
            }

            let voice_bottom = row_y - 6.0;

            // Barlines are drawn per voice group, so they never cross the
            // lyric lines belonging to a different sub-section.
            if !group.voices.is_empty() {
                for (pulse_id, pulse) in system.pulses.iter().enumerate() {
                    if pulse.measure_start {
                        let x = origin_x + xs[pulse_id];
                        Self::emit_line(svg, x, voice_top, x, voice_bottom, "barline")?;
                    }
                }
            }

            for lyric in &group.lyrics {
                let baseline = baseline(row_y, self.options.lyric_row_height, &fonts.lyric);

                self.emit_lyric_row(
                    svg,
                    fonts,
                    system,
                    &xs,
                    &usable_widths,
                    origin_x,
                    score.lead_width,
                    &group.voices,
                    lyric,
                    baseline,
                )?;

                row_y += self.options.lyric_row_height;
            }
        }

        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn emit_voice_row(
        &self,
        svg: &mut String,
        fonts: &Fonts,
        system: &System,
        xs: &[f32],
        usable_widths: &[f32],
        origin_x: f32,
        lead: f32,
        subsection: usize,
        voice: Voice,
        baseline: f32,
    ) -> Result<()> {
        let all_empty = system.pulses.iter().all(|pulse| {
            Self::find_voice_in_subsection(pulse, subsection, voice).is_none_or(|voice_pulse| {
                voice_pulse
                    .columns
                    .iter()
                    .all(|column| matches!(column.note, NoteKind::EmptyNote))
            })
        });
        if all_empty {
            eprintln!("EMIT all-empty voice {voice:?} subsection {subsection}");
        }

        let mut underline: Option<UnderlineState> = None;
        let mut last_note_end = origin_x;
        let normal_underline_y = baseline + fonts.solfa.descent() + 2.5;
        // Bottom octaves sit above the underline so the underline can keep the
        // same height on every note.
        let bottom_octave_y = normal_underline_y - fonts.octave.descent() - 1.0;

        for (pulse_id, pulse) in system.pulses.iter().enumerate() {
            let Some(voice_pulse) = Self::find_voice_in_subsection(pulse, subsection, voice) else {
                continue;
            };

            let usable_width = usable_widths
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
                let cell_x = origin_x + xs[pulse_id] + start * usable_width;
                let mut note_start = cell_x + lead;
                let mut note_end = note_start;

                let mut symbol = "";

                if column_id == 0 {
                    match pulse.accent {
                        PulseAccent::Strong => {}
                        PulseAccent::Medium => {
                            // A medium accent is printed as a short barline
                            // instead of an exclamation mark. Centre it on the
                            // visual middle of the note glyph, not on the text
                            // baseline (which would make it look too low).
                            let x = cell_x + lead / 2.0;
                            let note_center =
                                baseline - (fonts.solfa.ascent() - fonts.solfa.descent()) / 2.0;
                            let half = fonts.solfa.size() / 2.0;
                            Self::emit_line(
                                svg,
                                x,
                                note_center - half,
                                x,
                                note_center + half,
                                "medium-accent",
                            )?;
                        }
                        PulseAccent::Weak => symbol = ":",
                    }
                } else {
                    symbol = division_symbol(cursor, total);
                }

                if !symbol.is_empty() {
                    Self::emit_text(
                        svg,
                        cell_x + lead / 2.0,
                        baseline,
                        symbol,
                        "divider",
                        "middle",
                    )?;
                }

                if !matches!(column.note, NoteKind::EmptyNote) {
                    let (text, octave) = note_text(&column.note);
                    let glyph_width = fonts.solfa.width(&text);
                    let note_leading = if matches!(column.note, NoteKind::ProlongedNote) {
                        lead + Self::prolonged_leading(fonts)
                    } else {
                        lead
                    };
                    let glyph_start = cell_x + note_leading;
                    let glyph_center = glyph_start + glyph_width / 2.0;

                    note_start = glyph_start;
                    note_end = glyph_start + glyph_width;
                    last_note_end = note_end;

                    Self::emit_text(svg, glyph_center, baseline, &text, "solfa", "middle")?;

                    if let Some(octave) = octave {
                        let octave_x = glyph_center + glyph_width / 2.0 + OCTAVE_GAP;
                        let octave_y = if octave > 0 {
                            baseline - fonts.solfa.size() * 0.45
                        } else {
                            bottom_octave_y
                        };

                        Self::emit_text(
                            svg,
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
                    Self::emit_line(
                        svg,
                        open.start,
                        normal_underline_y,
                        note_end,
                        normal_underline_y,
                        "underline",
                    )?;
                }

                cursor += duration;
            }
        }

        if let Some(open) = underline {
            Self::emit_line(
                svg,
                open.start,
                normal_underline_y,
                last_note_end,
                normal_underline_y,
                "underline",
            )?;
        }

        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn emit_lyric_row(
        &self,
        svg: &mut String,
        fonts: &Fonts,
        system: &System,
        xs: &[f32],
        usable_widths: &[f32],
        origin_x: f32,
        lead: f32,
        voices: &[Voice],
        group: &SystemLyricGroup,
        baseline: f32,
    ) -> Result<()> {
        for token in &group.tokens {
            if token.text.is_empty() {
                continue;
            }

            let end_width = usable_widths
                .get(token.end)
                .copied()
                .unwrap_or(system.pulses[token.end].pulse_width);
            let end_x = origin_x + xs[token.end] + token.end_unit * end_width;
            let x = Self::lyric_glyph_start(
                system,
                xs,
                usable_widths,
                origin_x,
                lead,
                voices,
                group.subsection,
                token,
            )
            .unwrap_or_else(|| {
                let start_width = usable_widths
                    .get(token.start)
                    .copied()
                    .unwrap_or(system.pulses[token.start].pulse_width);
                origin_x + xs[token.start] + token.start_unit * start_width + lead
            });
            let end_x = end_x.max(x);

            // Lyrics are left-aligned at the first note they belong to. A
            // centred word could start before the note and spill into the
            // preceding accent or grid cell.
            Self::emit_text(svg, x, baseline, &token.text, "lyric", "start")?;

            if token.underline {
                let width = fonts.lyric.width(&token.text).max(1.0);
                let available = (end_x - x).max(1.0);
                let width = width.min(available);

                Self::emit_line(
                    svg,
                    x,
                    baseline + fonts.lyric.descent() + 1.0,
                    x + width,
                    baseline + fonts.lyric.descent() + 1.0,
                    "underline",
                )?;
            }
        }

        Ok(())
    }

    /// Find the left edge of the note that a lyric token starts on.
    ///
    /// This keeps lyrics attached to the note cell even when the note is a
    /// prolonged dash, which has extra leading space.
    #[allow(clippy::too_many_arguments)]
    fn lyric_glyph_start(
        system: &System,
        xs: &[f32],
        usable_widths: &[f32],
        origin_x: f32,
        lead: f32,
        voices: &[Voice],
        subsection: usize,
        token: &SystemLyricToken,
    ) -> Option<f32> {
        let pulse = system.pulses.get(token.start)?;
        let usable_width = usable_widths
            .get(token.start)
            .copied()
            .unwrap_or(pulse.pulse_width);

        for voice in voices {
            let Some(voice_pulse) = Self::find_voice_in_subsection(pulse, subsection, *voice)
            else {
                continue;
            };
            let total = voice_pulse
                .columns
                .iter()
                .map(|column| column.duration as u32)
                .sum::<u32>()
                .max(1);
            let mut cursor = 0u32;

            for column in &voice_pulse.columns {
                let start = cursor as f32 / total as f32;

                if !matches!(column.note, NoteKind::EmptyNote)
                    && (start - token.start_unit).abs() < 0.001
                {
                    // The lyric belongs to the note cell, not to the visual
                    // glyph offset used to detach a prolonged dash from the
                    // preceding accent.
                    return Some(origin_x + xs[token.start] + start * usable_width + lead);
                }

                cursor += column.duration as u32;
            }
        }

        None
    }

    fn find_voice_in_subsection<'a, 'b>(
        pulse: &'a SystemPulse<'b>,
        subsection: usize,
        voice: Voice,
    ) -> Option<&'b PulseIr> {
        pulse
            .subsections
            .get(subsection)?
            .voices
            .iter()
            .find(|(current, _)| *current == voice)
            .map(|(_, voice_pulse)| *voice_pulse)
    }

    fn pulse_offsets(system: &System) -> Vec<f32> {
        let mut result = Vec::with_capacity(system.pulses.len());
        let mut x = 0.0;

        for pulse in &system.pulses {
            result.push(x);
            x += pulse.pulse_width;
        }

        result
    }

    /// Keep a small gap at the right edge of every pulse so octave digits do
    /// not touch the barline at the next measure boundary.
    fn grid_right_padding(fonts: &Fonts) -> f32 {
        ["1", "2", "3", "4", "5", "6", "7"]
            .into_iter()
            .map(|digit| fonts.octave.width(digit))
            .fold(0.0f32, f32::max)
            + 2.0
    }

    /// Width of the pulse that notes may actually use. Wide glyphs near the
    /// end of a pulse can be wider than their duration-based cell, so the
    /// usable width is reduced until every note and octave digit ends before
    /// the next barline.
    fn pulse_usable_width(pulse: &SystemPulse, fonts: &Fonts, lead: f32) -> f32 {
        let mut usable = (pulse.pulse_width - Self::grid_right_padding(fonts)).max(0.0);

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
                        let extent = Self::note_extent(&column.note, fonts, lead);
                        let available = pulse.pulse_width - extent - 3.0;

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

    fn prolonged_leading(fonts: &Fonts) -> f32 {
        fonts.solfa.width(" ").max(2.5)
    }

    fn note_extent(note: &NoteKind, fonts: &Fonts, lead: f32) -> f32 {
        match note {
            NoteKind::Note(_) => {
                let (text, octave) = note_text(note);
                let base = fonts.solfa.width(&text);
                let octave = octave
                    .map(|octave| fonts.octave.width(&octave.abs().to_string()) / 2.0 + OCTAVE_GAP)
                    .unwrap_or(0.0);

                lead + base + octave
            }
            NoteKind::ProlongedNote => {
                lead + Self::prolonged_leading(fonts) + fonts.solfa.width("-")
            }
            NoteKind::EmptyNote => lead,
        }
    }

    fn layout_header(&self, header: &HeaderView, fonts: &Fonts) -> HeaderLayout {
        let max_width = self.options.printable_width();
        let mut lines = Vec::new();
        let mut y = 0.0;
        let mut first = true;

        if let Some(title) = &header.title {
            push_header_lines(
                &mut lines,
                &mut y,
                &mut first,
                title,
                HeaderKind::Title,
                &fonts.title,
                max_width,
                fonts.title.line_height() * 1.08,
                0.0,
            );
        }

        if !header.authors.is_empty() {
            let text = header.authors.join("  ·  ");

            push_header_lines(
                &mut lines,
                &mut y,
                &mut first,
                &text,
                HeaderKind::Name,
                &fonts.name,
                max_width,
                fonts.name.line_height(),
                6.0,
            );
        }

        let meta = [
            header.key.as_ref().map(|key| format!("Key: {key}")),
            header.time.as_ref().map(|time| format!("Time: {time}")),
            header.tempo.as_ref().map(|tempo| format!("Tempo: {tempo}")),
            header.meter.as_ref().map(|meter| format!("Meter: {meter}")),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join("    ");

        if !meta.is_empty() {
            push_header_lines(
                &mut lines,
                &mut y,
                &mut first,
                &meta,
                HeaderKind::Meta,
                &fonts.meta,
                max_width,
                fonts.meta.line_height(),
                6.0,
            );
        }

        HeaderLayout {
            lines,
            height: if first { 0.0 } else { y + 8.0 },
        }
    }

    fn emit_header(&self, svg: &mut String, header: &HeaderLayout) -> Result<()> {
        let center = self.options.page.width / 2.0;

        for line in &header.lines {
            let class = match line.kind {
                HeaderKind::Title => "title",
                HeaderKind::Name => "name",
                HeaderKind::Meta => "meta",
            };

            Self::emit_text(
                svg,
                center,
                self.options.margin + line.y,
                &line.text,
                class,
                "middle",
            )?;
        }

        Ok(())
    }

    fn emit_defs(&self, svg: &mut String) -> Result<()> {
        let solfa = STANDARD.encode(SOLFA_FONT);
        let lyric = STANDARD.encode(LYRIC_FONT);
        let options = &self.options;

        writeln!(
            svg,
            r#"<defs>
  <style>
    @font-face {{
      font-family: '{SOLFA_FAMILY}';
      src: url('data:font/ttf;base64,{solfa}') format('truetype');
      font-weight: normal;
      font-style: normal;
    }}

    @font-face {{
      font-family: '{LYRIC_FAMILY}';
      src: url('data:font/ttf;base64,{lyric}') format('truetype');
      font-weight: normal;
      font-style: normal;
    }}

    svg {{
      background: #ffffff;
    }}

    text {{
      fill: #1a1a1a;
    }}

    .solfa {{
      font-family: '{SOLFA_FAMILY}', monospace;
      font-size: {solfa_size}px;
      font-weight: bold;
    }}

    .accent, .divider {{
      font-family: '{SOLFA_FAMILY}', monospace;
      font-size: {symbol_size}px;
      fill: #333333;
    }}

    .octave {{
      font-family: '{SOLFA_FAMILY}', monospace;
      font-size: {octave_size}px;
      font-weight: bold;
    }}

    .lyric {{
      font-family: '{LYRIC_FAMILY}', sans-serif;
      font-size: {lyric_size}px;
    }}

    .title {{
      font-family: '{LYRIC_FAMILY}', sans-serif;
      font-size: {title_size}px;
      font-weight: bold;
    }}

    .name {{
      font-family: '{LYRIC_FAMILY}', sans-serif;
      font-size: {name_size}px;
    }}

    .meta {{
      font-family: '{LYRIC_FAMILY}', sans-serif;
      font-size: {meta_size}px;
      fill: #444444;
    }}

    .page-number {{
      font-family: '{LYRIC_FAMILY}', sans-serif;
      font-size: {page_number_size}px;
      fill: #777777;
    }}

    .barline {{
      stroke: #222222;
      stroke-width: 1.2;
      stroke-linecap: square;
    }}

    .medium-accent {{
      stroke: #333333;
      stroke-width: 1.0;
      stroke-linecap: square;
    }}

    .underline {{
      stroke: #222222;
      stroke-width: 1.0;
      stroke-linecap: round;
    }}
  </style>
</defs>"#,
            solfa_size = options.solfa_font_size,
            symbol_size = options.solfa_font_size * 0.85,
            octave_size = options.octave_font_size,
            lyric_size = options.lyric_font_size,
            title_size = options.title_font_size,
            name_size = options.name_font_size,
            meta_size = options.meta_font_size,
            page_number_size = options.meta_font_size * 0.9,
        )?;

        Ok(())
    }

    fn emit_text(
        svg: &mut String,
        x: f32,
        y: f32,
        text: &str,
        class: &str,
        anchor: &str,
    ) -> Result<()> {
        writeln!(
            svg,
            r#"<text x="{x:.2}" y="{y:.2}" text-anchor="{anchor}" class="{class}">{}</text>"#,
            xml_escape(text),
        )?;

        Ok(())
    }

    fn emit_line(svg: &mut String, x1: f32, y1: f32, x2: f32, y2: f32, class: &str) -> Result<()> {
        writeln!(
            svg,
            r#"<line x1="{x1:.2}" y1="{y1:.2}" x2="{x2:.2}" y2="{y2:.2}" class="{class}"/>"#,
        )?;

        Ok(())
    }
}

struct UnderlineState {
    start: f32,
}

struct VoiceGroupLayout<'a> {
    subsection: usize,
    voices: Vec<Voice>,
    lyrics: Vec<&'a SystemLyricGroup>,
}

#[derive(Debug)]
struct Page {
    systems: Vec<PlacedSystem>,
}

impl Default for Page {
    fn default() -> Self {
        Self {
            systems: Vec::new(),
        }
    }
}

#[derive(Debug)]
struct PlacedSystem {
    index: usize,
    y: f32,
}

#[derive(Debug, Clone, Copy)]
enum HeaderKind {
    Title,
    Name,
    Meta,
}

#[derive(Debug)]
struct HeaderLine {
    text: String,
    kind: HeaderKind,
    y: f32,
}

#[derive(Debug)]
struct HeaderLayout {
    lines: Vec<HeaderLine>,
    height: f32,
}

#[allow(clippy::too_many_arguments)]
fn push_header_lines(
    lines: &mut Vec<HeaderLine>,
    y: &mut f32,
    first: &mut bool,
    text: &str,
    kind: HeaderKind,
    font: &FontMetrics,
    max_width: f32,
    line_height: f32,
    gap: f32,
) {
    for line in wrap_text(text, max_width, font) {
        if !*first {
            *y += gap;
        }

        *y += line_height;
        lines.push(HeaderLine {
            text: line,
            kind,
            y: *y,
        });
        *first = false;
    }
}

fn wrap_text(text: &str, max_width: f32, font: &FontMetrics) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();

    for word in text.split_whitespace() {
        let candidate = if current.is_empty() {
            word.to_string()
        } else {
            format!("{current} {word}")
        };

        if current.is_empty() || font.width(&candidate) <= max_width {
            current = candidate;
        } else {
            lines.push(std::mem::take(&mut current));
            current = word.to_string();
        }
    }

    if !current.is_empty() {
        lines.push(current);
    }

    if lines.is_empty() {
        lines.push(String::new());
    }

    lines
}

fn baseline(row_y: f32, row_height: f32, font: &FontMetrics) -> f32 {
    let content = font.ascent() + font.descent();
    row_y + (row_height - content) / 2.0 + font.ascent()
}

fn note_text(note: &NoteKind) -> (String, Option<i8>) {
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

/// Reconstruct the printed divider from the resolved durations.
///
/// The source may omit dividers (`d r` is the same as `d .r` in a 2-subdivision
/// pulse), so the printed form is derived from the position inside the pulse.
fn division_symbol(cursor: u32, total: u32) -> &'static str {
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

fn xml_escape(input: &str) -> String {
    let mut escaped = String::with_capacity(input.len());

    for ch in input.chars() {
        match ch {
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '&' => escaped.push_str("&amp;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&apos;"),
            _ => escaped.push(ch),
        }
    }

    escaped
}

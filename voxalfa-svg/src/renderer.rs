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
    layout::*,
    locale::Locale,
    model::{HeaderView, ResolvedScore, System, SystemLyricToken, SystemPulse},
    options::RenderOptions,
    resolver,
};

const INITIAL_SVG_CAPACITY: usize = 32 * 1024;

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

        let system_layouts = score
            .systems
            .iter()
            .map(|system| self.layout_system(system))
            .collect::<Vec<_>>();

        let pages = self.paginate(&system_layouts, header.height);
        let page_count = pages.len().max(1);
        let total_height = self.options.page.height * page_count as f32;

        let mut svg = String::with_capacity(INITIAL_SVG_CAPACITY);

        writeln!(
            svg,
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {:.2} {:.2}" width="{:.2}px" height="{:.2}px">"#,
            self.options.page.width, total_height, self.options.page.width, total_height,
        )?;

        self.emit_defs(&mut svg)?;

        for (page_id, page) in pages.iter().enumerate() {
            let offset_y = page_id as f32 * self.options.page.height;

            let body_width = page
                .systems
                .iter()
                .map(|placed| Self::system_width(&score.systems[placed.index]))
                .fold(0.0, f32::max);

            let body_offset = ((self.options.printable_width() - body_width) / 2.0).max(0.0);

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
                let placement = SystemPlacement {
                    system: &score.systems[placed.index],
                    layout: &system_layouts[placed.index],
                    y: placed.y,
                    body_offset,
                };

                self.emit_system(&mut svg, &fonts, &score, placement)?;
            }

            if page_count > 1 {
                writeln!(
                    svg,
                    r#"<text x="{:.2}" y="{:.2}" text-anchor="middle" class="page-number">{} / {}</text>"#,
                    self.options.page.width / 2.0,
                    self.options.page.height - self.options.min_margin / 2.0,
                    page_id + 1,
                    page_count,
                )?;
            }

            writeln!(svg, "</g>")?;
        }

        writeln!(svg, "</svg>")?;

        Ok(svg)
    }

    fn paginate(&self, layouts: &[SystemLayout], header_height: f32) -> Vec<Page> {
        let bottom = self.options.page.height - self.options.min_margin;

        let mut pages = vec![Page::default()];
        let mut y = self.options.min_margin;

        y += header_height;

        if header_height > 0.0 {
            y += self.options.header_gap;
        }

        for (index, layout) in layouts.iter().enumerate() {
            if y + layout.height > bottom && !pages.last().unwrap().systems.is_empty() {
                pages.push(Page::default());
                y = self.options.min_margin;
            }

            pages
                .last_mut()
                .unwrap()
                .systems
                .push(PlacedSystem { index, y });

            y += layout.height + self.options.system_gap;
        }

        pages
    }

    fn layout_system(&self, system: &System) -> SystemLayout {
        let mut lyric_rows: Vec<Vec<(usize, usize)>> = vec![Vec::new(); system.voices.len()];

        for (group_index, group) in system.lyric_groups.iter().enumerate() {
            let Some(last_voice) = group
                .voices
                .iter()
                .filter_map(|voice| system.voices.iter().position(|item| item == voice))
                .max()
            else {
                continue;
            };

            for (verse_index, verse) in group.verses.iter().enumerate() {
                if !verse.is_empty() {
                    lyric_rows[last_voice].push((group_index, verse_index));
                }
            }
        }

        let mut specs = Vec::new();
        let mut kinds = Vec::new();

        for (voice_index, voice) in system.voices.iter().enumerate() {
            specs.push(RowSpec {
                height: self.options.voice_row_height,
                gap_before: 0.0,
            });

            kinds.push(SystemRowKind::Voice(*voice));

            for &(group_index, verse_index) in &lyric_rows[voice_index] {
                specs.push(RowSpec {
                    height: self.options.lyric_row_height,
                    gap_before: 0.0,
                });
                kinds.push(SystemRowKind::Lyric {
                    group_index,
                    verse_index,
                });
            }
        }

        let column = layout_column(&specs);

        let rows = kinds
            .into_iter()
            .enumerate()
            .map(|(index, kind)| SystemRow {
                y: column.ys[index],
                height: specs[index].height,
                kind,
            })
            .collect();

        SystemLayout {
            rows,
            height: column.height,
        }
    }

    fn emit_system(
        &self,
        svg: &mut String,
        fonts: &Fonts,
        score: &ResolvedScore,
        placement: SystemPlacement<'_>,
    ) -> Result<()> {
        let pulse_width = placement
            .system
            .pulses
            .first()
            .map(|pulse| pulse.pulse_width)
            .unwrap_or(0.0);

        let origin_x = self.options.min_margin
            + placement.body_offset
            + placement.system.grid_offset as f32 * pulse_width;

        let mut render = SystemRender::new(svg, fonts, placement, score.lead_width, origin_x);

        render.emit()
    }

    fn find_voice<'a>(pulse: &SystemPulse<'a>, voice: Voice) -> Option<&'a PulseIr> {
        pulse.subsections.iter().find_map(|sub| {
            sub.voices
                .iter()
                .find(|(current, _)| *current == voice)
                .map(|(_, voice_pulse)| *voice_pulse)
        })
    }

    fn system_width(system: &System) -> f32 {
        let leading_width = system
            .pulses
            .first()
            .map(|pulse| pulse.pulse_width)
            .unwrap_or(0.0)
            * system.grid_offset as f32;

        let pulses_width = system
            .pulses
            .iter()
            .map(|pulse| pulse.pulse_width)
            .sum::<f32>();

        leading_width + pulses_width
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

    fn octave_width(fonts: &Fonts) -> f32 {
        ["1", "2", "3", "4", "5", "6", "7"]
            .into_iter()
            .map(|digit| fonts.octave.width(digit))
            .fold(0.0, f32::max)
            + GRID_RIGHT_PADDING
    }

    fn pulse_usable_width(pulse: &SystemPulse, fonts: &Fonts, lead: f32) -> f32 {
        let mut usable = (pulse.pulse_width - Self::octave_width(fonts)).max(0.0);

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
                        let available = pulse.pulse_width - extent - NOTE_BARLINE_CLEARANCE;

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
        fonts.solfa.width(" ").max(PROLONGED_LEADING_MIN)
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
        let left = self.options.min_margin;
        let right = left + self.options.printable_width() - HEADER_RIGHT_INSET;
        let center = self.options.page.width / 2.0;
        let mut rows = Vec::new();

        if let Some(title) = &header.title {
            for line in wrap_text(title, self.options.printable_width(), &fonts.title) {
                rows.push(HeaderRow {
                    texts: vec![HeaderText {
                        text: line,
                        kind: HeaderKind::Title,
                        x: center,
                        anchor: TextAnchor::Middle,
                    }],
                    height: fonts.title.line_height() * TITLE_LINE_HEIGHT_FACTOR,
                    gap_before: 0.0,
                });
            }
        }

        let authors = header.authors.join(", ");
        let composers = header.composers.join(", ");

        if !authors.is_empty() || !composers.is_empty() {
            let mut texts = Vec::new();

            if !authors.is_empty() {
                texts.push(HeaderText {
                    text: authors,
                    kind: HeaderKind::Name,
                    x: left,
                    anchor: TextAnchor::Start,
                });
            }

            if !composers.is_empty() {
                texts.push(HeaderText {
                    text: composers,
                    kind: HeaderKind::Name,
                    x: right,
                    anchor: TextAnchor::End,
                });
            }

            let gap_before = if rows.is_empty() {
                0.0
            } else {
                HEADER_TITLE_BOTTOM_GAP
            };

            rows.push(HeaderRow {
                texts,
                height: fonts.name.line_height(),
                gap_before,
            });
        }

        let locale = Locale::from_language(header.language.as_deref());
        let key = header.key.as_ref().map(|key| locale.key_label(key));

        let time_x = key
            .as_ref()
            .map(|text| left + fonts.meta.width(text) + HEADER_KEY_RIGHT_MARGIN)
            .unwrap_or(left);

        if key.is_some() || header.meter.is_some() || header.time.is_some() {
            let mut texts = Vec::new();

            if let Some(text) = key {
                texts.push(HeaderText {
                    text,
                    kind: HeaderKind::Meta,
                    x: left,
                    anchor: TextAnchor::Start,
                });
            }

            if let Some(time) = &header.time {
                texts.push(HeaderText {
                    text: time.clone(),
                    kind: HeaderKind::Meta,
                    x: time_x,
                    anchor: TextAnchor::Start,
                });
            }

            if let Some(meter) = &header.meter {
                texts.push(HeaderText {
                    text: meter.clone(),
                    kind: HeaderKind::Meta,
                    x: center,
                    anchor: TextAnchor::Middle,
                });
            }

            let gap_before = if rows.is_empty() {
                0.0
            } else {
                HEADER_GROUP_GAP
            };

            rows.push(HeaderRow {
                texts,
                height: fonts.meta.line_height(),
                gap_before,
            });
        }

        if rows.is_empty() {
            return HeaderLayout {
                lines: Vec::new(),
                height: 0.0,
            };
        }

        let mut lines = Vec::new();
        let mut y = 0.0;

        for row in rows {
            y += row.gap_before;
            let baseline = y + row.height;

            for text in row.texts {
                lines.push(HeaderLine {
                    text: text.text,
                    kind: text.kind,
                    x: text.x,
                    y: baseline,
                    anchor: text.anchor,
                });
            }

            y += row.height;
        }

        HeaderLayout {
            lines,
            height: y + HEADER_BOTTOM_PADDING,
        }
    }

    fn emit_header(&self, svg: &mut String, header: &HeaderLayout) -> Result<()> {
        for line in &header.lines {
            let class = match line.kind {
                HeaderKind::Title => "title",
                HeaderKind::Name => "name",
                HeaderKind::Meta => "meta",
            };

            Self::emit_text(
                svg,
                line.x,
                self.options.min_margin + line.y,
                &line.text,
                class,
                line.anchor.as_str(),
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
      stroke-width: {barline_stroke};
      stroke-linecap: square;
    }}

    .medium-accent {{
      stroke: #333333;
      stroke-width: {accent_stroke};
      stroke-linecap: square;
    }}

    .underline {{
      stroke: #222222;
      stroke-width: {underline_stroke};
      stroke-linecap: round;
    }}
  </style>
</defs>"#,
            solfa_size = options.solfa_font_size,
            symbol_size = options.solfa_font_size * SYMBOL_FONT_SCALE,
            octave_size = options.octave_font_size,
            lyric_size = options.lyric_font_size,
            title_size = options.title_font_size,
            name_size = options.name_font_size,
            meta_size = options.meta_font_size,
            page_number_size = options.meta_font_size * PAGE_NUMBER_FONT_SCALE,
            barline_stroke = BARLINE_STROKE_WIDTH,
            accent_stroke = ACCENT_STROKE_WIDTH,
            underline_stroke = UNDERLINE_STROKE_WIDTH,
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

impl<'a> SystemRender<'a> {
    fn new(
        svg: &'a mut String,
        fonts: &'a Fonts,
        placement: SystemPlacement<'a>,
        lead: f32,
        origin_x: f32,
    ) -> Self {
        let xs = Renderer::pulse_offsets(placement.system);
        let usable_widths = placement
            .system
            .pulses
            .iter()
            .map(|pulse| Renderer::pulse_usable_width(pulse, fonts, lead))
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

        let system_top = layout
            .rows
            .iter()
            .find_map(|row| match row.kind {
                SystemRowKind::Voice(_) => Some(y + row.y + BARLINE_BLOCK_INSET),
                SystemRowKind::Lyric { .. } => None,
            })
            .unwrap_or(y + BARLINE_BLOCK_INSET);

        let system_bottom = y + layout.height - BARLINE_BLOCK_INSET;

        for (pulse_id, pulse) in system.pulses.iter().enumerate() {
            if !pulse.section_end {
                continue;
            }

            let x = self.origin_x + self.xs[pulse_id] + pulse.pulse_width;

            Renderer::emit_line(
                self.svg,
                x - DOUBLE_BARLINE_GAP,
                system_top,
                x - DOUBLE_BARLINE_GAP,
                system_bottom,
                "barline",
            )?;

            Renderer::emit_line(self.svg, x, system_top, x, system_bottom, "barline")?;
        }

        Ok(())
    }

    fn emit_barlines(&mut self, top: f32, bottom: f32) -> Result<()> {
        let system = self.system;
        let origin_x = self.origin_x;

        for (pulse_id, pulse) in system.pulses.iter().enumerate() {
            if pulse.measure_start {
                let x = origin_x + self.xs[pulse_id];
                Renderer::emit_line(self.svg, x, top, x, bottom, "barline")?;
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
            let Some(voice_pulse) = Renderer::find_voice(pulse, voice) else {
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
                } else {
                    symbol = division_symbol(cursor, total);
                }

                if !symbol.is_empty() {
                    Renderer::emit_text(
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
                        NoteKind::ProlongedNote => {
                            self.lead + Renderer::prolonged_leading(self.fonts)
                        }
                        _ => self.lead,
                    };

                    let glyph_start = cell_x + note_leading;
                    let glyph_center = glyph_start + glyph_width / 2.0;

                    note_start = glyph_start;
                    note_end = glyph_start + glyph_width;
                    last_note_end = note_end;

                    Renderer::emit_text(
                        self.svg,
                        glyph_center,
                        baseline,
                        &text,
                        "solfa",
                        "middle",
                    )?;

                    if let Some(octave) = octave {
                        let octave_x = glyph_center + glyph_width / 2.0 + OCTAVE_GAP;

                        let octave_y = if octave > 0 {
                            baseline - self.fonts.solfa.size() * TOP_OCTAVE_OFFSET_FACTOR
                        } else {
                            octave_y
                        };

                        Renderer::emit_text(
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
                    Renderer::emit_line(
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
            Renderer::emit_line(
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

                Renderer::emit_line(
                    self.svg,
                    x,
                    note_center - half,
                    x,
                    note_center + half,
                    "medium-accent",
                )?;
            }
            PulseAccent::Weak => {
                Renderer::emit_text(
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

            Renderer::emit_text(self.svg, x, baseline, &token.text, "lyric", "start")?;

            if token.underline {
                let width = fonts.lyric.width(&token.text).max(MIN_UNDERLINE_WIDTH);
                let available = (end_x - x).max(MIN_UNDERLINE_WIDTH);
                let width = width.min(available);

                Renderer::emit_line(
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

struct SystemLayout {
    rows: Vec<SystemRow>,
    height: f32,
}

struct SystemPlacement<'a> {
    system: &'a System<'a>,
    layout: &'a SystemLayout,
    body_offset: f32,
    y: f32,
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

struct SystemRow {
    y: f32,
    height: f32,
    kind: SystemRowKind,
}

enum SystemRowKind {
    Voice(Voice),
    Lyric {
        group_index: usize,
        verse_index: usize,
    },
}

#[derive(Debug, Clone, Copy)]
enum HeaderKind {
    Title,
    Name,
    Meta,
}

#[derive(Debug, Clone, Copy)]
enum TextAnchor {
    Start,
    Middle,
    End,
}

impl TextAnchor {
    fn as_str(self) -> &'static str {
        match self {
            Self::Start => "start",
            Self::Middle => "middle",
            Self::End => "end",
        }
    }
}

#[derive(Debug)]
struct HeaderLine {
    text: String,
    kind: HeaderKind,
    x: f32,
    y: f32,
    anchor: TextAnchor,
}

struct HeaderText {
    text: String,
    kind: HeaderKind,
    x: f32,
    anchor: TextAnchor,
}

struct HeaderRow {
    texts: Vec<HeaderText>,
    height: f32,
    gap_before: f32,
}

#[derive(Debug)]
struct HeaderLayout {
    lines: Vec<HeaderLine>,
    height: f32,
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

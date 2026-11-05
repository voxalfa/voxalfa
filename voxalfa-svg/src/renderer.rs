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
    fonts::{
        FontMetrics, Fonts, LYRIC_FAMILY, LYRIC_FONT, SERIF_BOLD_ITALIC_FONT, SERIF_FAMILY,
        SERIF_FONT, SERIF_ITALIC_FONT, SOLFA_FAMILY, SOLFA_FONT, TEXT_BOLD_FONT, TEXT_FAMILY,
        TEXT_FONT,
    },
    layout::*,
    locale::Locale,
    model::{
        DynamicSpanKind, HeaderView, MarkerGlyph, MarkerKind, ResolvedScore, System,
        SystemLyricToken, SystemMarker, SystemPulse,
    },
    options::RenderOptions,
    resolver,
    symbols::{
        FERMATA_GROUP, FERMATA_HEIGHT, FERMATA_WIDTH, MARCATO_HEIGHT, MARCATO_PATH, MARCATO_WIDTH,
        SEGNO_ASPECT_RATIO, SEGNO_HEIGHT_SCALE, SEGNO_PATH, SEGNO_PATH_TRANSFORM, STACCATO_PATH,
        STACCATO_SIZE,
    },
};

const INITIAL_SVG_CAPACITY: usize = 32 * 1024;
const MARKER_SAME_NOTE_EPSILON: f32 = 0.001;

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
            .map(|system| self.layout_system(system, &fonts, score.lead_width))
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

    fn layout_system(&self, system: &System, fonts: &Fonts, lead: f32) -> SystemLayout {
        let has_voltas = !system.voltas.is_empty();

        let (mut jumps, jump_height) = if has_voltas {
            self.layout_jumps(system, fonts, lead)
        } else {
            (Vec::new(), 0.0)
        };

        let (mut voltas, volta_height) = self.layout_voltas(system, fonts, lead);
        let (mut markers, marker_height) = self.layout_markers(system, fonts, lead, has_voltas);

        let dynamic_center = markers
            .iter()
            .find(|marker| marker.kind == MarkerKind::Dynamic)
            .map(|marker| marker.y - (fonts.dynamic.ascent() - fonts.dynamic.descent()) / 2.0);

        let (mut dynamic_spans, dynamic_span_height) =
            self.layout_dynamic_spans(system, fonts, lead, dynamic_center, marker_height);

        // A jump above a volta shares the volta's text row so it sits directly
        // on top of the bracket line instead of floating above the ending label.
        let jump_offset = if voltas.is_empty() { jump_height } else { 0.0 };

        let volta_top = if voltas.is_empty() {
            0.0
        } else {
            VOLTA_TOP_GAP
        };

        for jump in &mut jumps {
            jump.y += volta_top;
        }

        for volta in &mut voltas {
            volta.text_y += volta_top + jump_offset;
            volta.line_y += volta_top + jump_offset;
        }

        let marker_shift = volta_top + jump_offset + volta_height;

        for marker in &mut markers {
            marker.y += marker_shift;
        }

        for span in &mut dynamic_spans {
            span.center_y += marker_shift;
            span.baseline += marker_shift;
        }

        jumps.append(&mut markers);

        let markers = jumps;

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
                y: column.ys[index] + marker_shift + marker_height + dynamic_span_height,
                height: specs[index].height,
                kind,
            })
            .collect();

        SystemLayout {
            rows,
            markers,
            dynamic_spans,
            voltas,
            height: column.height
                + marker_height
                + volta_height
                + jump_offset
                + volta_top
                + dynamic_span_height,
        }
    }

    fn layout_jumps(&self, system: &System, fonts: &Fonts, lead: f32) -> (Vec<PlacedMarker>, f32) {
        let xs = Self::pulse_offsets(system);
        let usable_widths = system
            .pulses
            .iter()
            .map(|pulse| Self::pulse_usable_width(pulse, fonts, lead))
            .collect::<Vec<_>>();
        let mut jumps = Vec::new();

        for marker in system
            .markers
            .iter()
            .filter(|marker| marker.kind == MarkerKind::Jump)
        {
            let font = marker_font(marker.kind, fonts);
            let width = match &marker.glyph {
                MarkerGlyph::Text(text) => font.width(text),
                MarkerGlyph::Segno => Self::segno_width(fonts),
                MarkerGlyph::Fermata | MarkerGlyph::Staccato | MarkerGlyph::Marcato => {
                    touch_width(&marker.glyph, fonts)
                }
            };

            jumps.push(PlacedMarker {
                kind: marker.kind,
                glyph: marker.glyph.clone(),
                width,
                anchor: TextAnchor::End,
                x: xs[marker.pulse] + marker.unit * usable_widths[marker.pulse] + lead,
                y: font.ascent(),
            });
        }

        if jumps.is_empty() {
            return (Vec::new(), 0.0);
        }

        spread_markers(&mut jumps);
        let height = fonts.marker_bold.line_height() + MARKER_BOTTOM_GAP;

        (jumps, height)
    }

    fn layout_voltas(&self, system: &System, fonts: &Fonts, lead: f32) -> (Vec<PlacedVolta>, f32) {
        if system.voltas.is_empty() {
            return (Vec::new(), 0.0);
        }

        let xs = Self::pulse_offsets(system);
        let text_y = fonts.marker_bold.ascent();
        let line_y = fonts.marker_bold.line_height() + VOLTA_TEXT_GAP;
        let mut voltas = Vec::new();

        for volta in &system.voltas {
            if volta.start >= volta.end {
                continue;
            }

            let start_x = xs[volta.start] + lead;
            let end_x = xs[volta.end - 1] + system.pulses[volta.end - 1].pulse_width;
            let text = match &volta.label {
                Some(label) => format!("{}. {label}", volta.number),
                None => format!("{}.", volta.number),
            };

            voltas.push(PlacedVolta {
                start_x,
                end_x,
                text,
                text_x: start_x + VOLTA_TEXT_INSET,
                text_y,
                line_y,
            });
        }

        if voltas.is_empty() {
            return (Vec::new(), 0.0);
        }

        let height = line_y + VOLTA_END_TICK + VOLTA_BOTTOM_GAP;

        (voltas, height)
    }

    fn layout_dynamic_spans(
        &self,
        system: &System,
        fonts: &Fonts,
        lead: f32,
        dynamic_center: Option<f32>,
        marker_height: f32,
    ) -> (Vec<PlacedDynamicSpan>, f32) {
        if system.dynamic_spans.is_empty() {
            return (Vec::new(), 0.0);
        }

        let xs = Self::pulse_offsets(system);
        let usable_widths = system
            .pulses
            .iter()
            .map(|pulse| Self::pulse_usable_width(pulse, fonts, lead))
            .collect::<Vec<_>>();
        let row_height = fonts.dynamic.line_height().max(DYNAMIC_HAIRPIN_HEIGHT);
        let (center_y, height) = match dynamic_center {
            Some(center_y) => (center_y, 0.0),
            None => (
                marker_height + row_height / 2.0,
                row_height + DYNAMIC_SPAN_BOTTOM_GAP,
            ),
        };
        let baseline = center_y + (fonts.dynamic.ascent() - fonts.dynamic.descent()) / 2.0;
        let mut spans = Vec::new();

        for span in &system.dynamic_spans {
            let start_x = xs[span.start] + span.start_unit * usable_widths[span.start] + lead;
            let end_x = xs[span.end] + span.end_unit * usable_widths[span.end] + lead;

            if span.fits {
                let start_x = start_x + DYNAMIC_SPAN_INSET;
                let end_x = (end_x - DYNAMIC_SPAN_INSET).max(start_x + 1.0);

                spans.push(PlacedDynamicSpan {
                    kind: span.kind,
                    start_x,
                    end_x,
                    center_y,
                    baseline,
                    label: None,
                    label_width: 0.0,
                });
            } else {
                let label = match span.kind {
                    DynamicSpanKind::Crescendo => "cresc.",
                    DynamicSpanKind::Decrescendo => "decresc.",
                };

                spans.push(PlacedDynamicSpan {
                    kind: span.kind,
                    start_x,
                    end_x,
                    center_y,
                    baseline,
                    label: Some(label.to_string()),
                    label_width: fonts.dynamic.width(label),
                });
            }
        }

        (spans, height)
    }

    /// Place section markers above the note block. Time signatures get the
    /// top lane; every other marker shares the lane below it.
    fn layout_markers(
        &self,
        system: &System,
        fonts: &Fonts,
        lead: f32,
        separate_jumps: bool,
    ) -> (Vec<PlacedMarker>, f32) {
        if system.markers.is_empty() {
            return (Vec::new(), 0.0);
        }

        let xs = Self::pulse_offsets(system);
        let usable_widths = system
            .pulses
            .iter()
            .map(|pulse| Self::pulse_usable_width(pulse, fonts, lead))
            .collect::<Vec<_>>();

        let note_center = |pulse_id: usize, unit: f32| -> Option<f32> {
            let pulse = system.pulses.get(pulse_id)?;
            let usable_width = usable_widths[pulse_id];

            for voice in &system.voices {
                let Some(voice_pulse) = Renderer::find_voice(pulse, *voice) else {
                    continue;
                };

                let total = voice_pulse
                    .columns
                    .iter()
                    .map(|column| column.duration as u32)
                    .sum::<u32>()
                    .max(1);
                let mut cursor = 0u32;

                for (index, column) in voice_pulse.columns.iter().enumerate() {
                    let start = cursor as f32 / total as f32;
                    cursor += column.duration as u32;
                    let end = cursor as f32 / total as f32;
                    let is_last = index + 1 == voice_pulse.columns.len();

                    if unit + MARKER_SAME_NOTE_EPSILON >= start
                        && (unit < end - MARKER_SAME_NOTE_EPSILON
                            || (is_last && unit <= end + MARKER_SAME_NOTE_EPSILON))
                    {
                        if matches!(column.note, NoteKind::EmptyNote) {
                            break;
                        }

                        let (text, _) = note_text(&column.note);
                        let glyph_width = fonts.solfa.width(&text);
                        let leading = match column.note {
                            NoteKind::ProlongedNote => lead + Renderer::prolonged_leading(fonts),
                            _ => lead,
                        };

                        return Some(
                            xs[pulse_id] + start * usable_width + leading + glyph_width / 2.0,
                        );
                    }
                }
            }

            None
        };

        let mut ordered = system
            .markers
            .iter()
            .filter(|marker| !separate_jumps || marker.kind != MarkerKind::Jump)
            .collect::<Vec<_>>();

        ordered.sort_by(|lhs, rhs| {
            lhs.pulse
                .cmp(&rhs.pulse)
                .then(lhs.unit.total_cmp(&rhs.unit))
        });

        let mut groups: Vec<Vec<&SystemMarker>> = Vec::new();

        for marker in ordered {
            if let Some(group) = groups.last_mut() {
                let first = group[0];

                if first.pulse == marker.pulse
                    && (first.unit - marker.unit).abs() < MARKER_SAME_NOTE_EPSILON
                {
                    group.push(marker);
                    continue;
                }
            }

            groups.push(vec![marker]);
        }

        let mut rows: Vec<Vec<PlacedMarker>> = Vec::new();

        for mut group in groups {
            group.sort_by_key(|marker| marker_priority(marker.kind));

            for (row, marker) in group.into_iter().enumerate() {
                let font = marker_font(marker.kind, fonts);
                let width = match &marker.glyph {
                    MarkerGlyph::Text(text) => font.width(text),
                    MarkerGlyph::Segno => Self::segno_width(fonts),
                    MarkerGlyph::Fermata | MarkerGlyph::Staccato | MarkerGlyph::Marcato => {
                        touch_width(&marker.glyph, fonts)
                    }
                };
                let fallback_x =
                    xs[marker.pulse] + marker.unit * usable_widths[marker.pulse] + lead;
                let x = match marker.kind {
                    MarkerKind::Dynamic | MarkerKind::Touch => {
                        note_center(marker.pulse, marker.unit).unwrap_or(fallback_x)
                    }
                    MarkerKind::Mark if matches!(&marker.glyph, MarkerGlyph::Segno) => {
                        note_center(marker.pulse, marker.unit).unwrap_or(fallback_x)
                    }
                    _ => fallback_x,
                };

                let placed = PlacedMarker {
                    kind: marker.kind,
                    glyph: marker.glyph.clone(),
                    width,
                    x,
                    y: 0.0,
                    anchor: match marker.kind {
                        MarkerKind::TimeSignature => TextAnchor::Middle,
                        MarkerKind::Key | MarkerKind::Tempo | MarkerKind::TempoNumber => {
                            TextAnchor::Start
                        }
                        MarkerKind::Dynamic | MarkerKind::Touch => TextAnchor::Middle,
                        MarkerKind::Jump => TextAnchor::End,
                        MarkerKind::Mark if marker.unit >= 0.5 => TextAnchor::End,
                        MarkerKind::Mark => TextAnchor::Middle,
                    },
                };

                if rows.len() <= row {
                    rows.resize_with(row + 1, Vec::new);
                }

                rows[row].push(placed);
            }
        }

        for row in &mut rows {
            spread_markers(row);
        }

        let mut top = 0.0;

        for row in &mut rows {
            if row.is_empty() {
                continue;
            }

            let row_height = row
                .iter()
                .map(|marker| marker_font(marker.kind, fonts).line_height())
                .fold(0.0, f32::max);

            for marker in row {
                let font = marker_font(marker.kind, fonts);
                marker.y = top + (row_height - font.line_height()) / 2.0 + font.ascent();
            }

            top += row_height;
        }

        if top == 0.0 {
            return (Vec::new(), 0.0);
        }

        let markers = rows.into_iter().flatten().collect();
        let height = top + MARKER_BOTTOM_GAP;

        (markers, height)
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
            + placement.system.pulse_offset as f32 * pulse_width;

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
            * system.pulse_offset as f32;

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

    fn segno_width(fonts: &Fonts) -> f32 {
        fonts.marker.line_height() * SEGNO_HEIGHT_SCALE * SEGNO_ASPECT_RATIO
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
            .map(|text| left + fonts.key.width(text) + HEADER_KEY_RIGHT_MARGIN)
            .unwrap_or(left);

        if key.is_some() || header.meter.is_some() || header.time.is_some() {
            let mut texts = Vec::new();

            if let Some(text) = key {
                texts.push(HeaderText {
                    text,
                    kind: HeaderKind::Key,
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
                height: fonts.meta.line_height().max(fonts.key.line_height()),
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
                HeaderKind::Key => "key",
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
        let text = STANDARD.encode(TEXT_FONT);
        let text_bold = STANDARD.encode(TEXT_BOLD_FONT);
        let serif = STANDARD.encode(SERIF_FONT);
        let serif_italic = STANDARD.encode(SERIF_ITALIC_FONT);
        let serif_bold_italic = STANDARD.encode(SERIF_BOLD_ITALIC_FONT);
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
      font-family: '{TEXT_FAMILY}';
      src: url('data:font/ttf;base64,{text}') format('truetype');
      font-weight: normal;
      font-style: normal;
    }}

    @font-face {{
      font-family: '{TEXT_FAMILY}';
      src: url('data:font/ttf;base64,{text_bold}') format('truetype');
      font-weight: bold;
      font-style: normal;
    }}

    @font-face {{
      font-family: '{SERIF_FAMILY}';
      src: url('data:font/ttf;base64,{serif}') format('truetype');
      font-weight: normal;
      font-style: normal;
    }}

    @font-face {{
      font-family: '{SERIF_FAMILY}';
      src: url('data:font/ttf;base64,{serif_italic}') format('truetype');
      font-weight: normal;
      font-style: italic;
    }}

    @font-face {{
      font-family: '{SERIF_FAMILY}';
      src: url('data:font/ttf;base64,{serif_bold_italic}') format('truetype');
      font-weight: bold;
      font-style: italic;
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
      font-family: '{TEXT_FAMILY}', sans-serif;
      font-size: {title_size}px;
      font-weight: bold;
    }}

    .name {{
      font-family: '{TEXT_FAMILY}', sans-serif;
      font-size: {name_size}px;
    }}

    .key {{
      font-family: '{TEXT_FAMILY}', sans-serif;
      font-size: {marker_size}px;
      font-weight: bold;
    }}

    .meta {{
      font-family: '{TEXT_FAMILY}', sans-serif;
      font-size: {meta_size}px;
      fill: #444444;
    }}

    .marker {{
      font-family: '{SERIF_FAMILY}', serif;
      font-size: {marker_size}px;
      font-weight: normal;
      fill: #333333;
    }}

    .marker-sans {{
      font-family: '{TEXT_FAMILY}', sans-serif;
      font-size: {marker_size}px;
      font-weight: normal;
      fill: #333333;
    }}

    .marker-bold {{
      font-family: '{TEXT_FAMILY}', sans-serif;
      font-size: {marker_size}px;
      font-weight: bold;
      fill: #333333;
    }}

    .marker-italic {{
      font-family: '{SERIF_FAMILY}', serif;
      font-size: {marker_size}px;
      font-style: italic;
      fill: #333333;
    }}

    .dynamic {{
      font-family: '{SERIF_FAMILY}', serif;
      font-size: {dynamic_size}px;
      font-weight: bold;
      font-style: italic;
    }}

    .volta {{
      stroke: #222222;
      stroke-width: {barline_stroke};
      stroke-linecap: square;
    }}

    .volta-text {{
      font-family: '{TEXT_FAMILY}', sans-serif;
      font-size: {marker_size}px;
      font-weight: bold;
      fill: #333333;
    }}

    .page-number {{
      font-family: '{TEXT_FAMILY}', sans-serif;
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

    .touch-fill {{
      fill: #222222;
      stroke: none;
    }}

    .dynamic-hairpin {{
      stroke: #222222;
      stroke-width: 1;
      stroke-linecap: round;
    }}

    .dynamic-dash {{
      stroke: #222222;
      stroke-width: 1;
      stroke-dasharray: 4 3;
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
            marker_size = options.marker_font_size,
            dynamic_size = options.marker_font_size * DYNAMIC_FONT_SCALE,
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

    fn emit_segno(svg: &mut String, x: f32, baseline: f32, fonts: &Fonts) -> Result<()> {
        let line_height = fonts.marker.line_height();
        let height = line_height * SEGNO_HEIGHT_SCALE;
        let scale = height / 3.0;
        let left = x - Self::segno_width(fonts) / 2.0;
        let top = baseline - fonts.marker.ascent() + (line_height - height) / 2.0;

        writeln!(
            svg,
            r##"<g class="marker marker-segno" transform="translate({left:.2} {top:.2}) scale({scale:.4})"><path transform="{SEGNO_PATH_TRANSFORM}" d="{SEGNO_PATH}" fill="#333333"/></g>"##,
        )?;

        Ok(())
    }

    fn emit_touch(
        svg: &mut String,
        glyph: &MarkerGlyph,
        x: f32,
        baseline: f32,
        fonts: &Fonts,
    ) -> Result<()> {
        let size = fonts.marker.size() * TOUCH_SYMBOL_SCALE;
        let bottom = baseline - 1.0;

        match glyph {
            MarkerGlyph::Staccato => {
                let width = size * 0.32;
                let scale = width / STACCATO_SIZE;
                let left = x - width / 2.0;
                let top = bottom - width;

                writeln!(
                    svg,
                    r##"<g class="touch-fill touch-staccato" transform="translate({left:.2} {top:.2}) scale({scale:.4})"><path d="{STACCATO_PATH}"/></g>"##,
                )?;
            }
            MarkerGlyph::Marcato => {
                let width = size * MARCATO_WIDTH_SCALE;
                let height = width * MARCATO_HEIGHT / MARCATO_WIDTH;
                let scale = width / MARCATO_WIDTH;
                let left = x - width / 2.0;
                let top = bottom - height;

                writeln!(
                    svg,
                    r##"<g class="touch-fill touch-marcato" transform="translate({left:.2} {top:.2}) scale({scale:.4})"><path d="{MARCATO_PATH}"/></g>"##,
                )?;
            }
            MarkerGlyph::Fermata => {
                let width = size * 1.7;
                let height = width * FERMATA_HEIGHT / FERMATA_WIDTH;
                let scale = width / FERMATA_WIDTH;
                let left = x - width / 2.0;
                let top = bottom - height;

                writeln!(
                    svg,
                    r##"<g class="touch-fermata" transform="translate({left:.2} {top:.2}) scale({scale:.4})">{FERMATA_GROUP}</g>"##,
                )?;
            }
            _ => {}
        }

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

        for volta in &layout.voltas {
            let start_x = self.origin_x + volta.start_x;
            let end_x = self.origin_x + volta.end_x;
            let line_y = y + volta.line_y;

            Renderer::emit_line(self.svg, start_x, line_y, end_x, line_y, "volta")?;
            Renderer::emit_line(
                self.svg,
                start_x,
                line_y,
                start_x,
                line_y + VOLTA_END_TICK,
                "volta",
            )?;
            Renderer::emit_line(
                self.svg,
                end_x,
                line_y,
                end_x,
                line_y + VOLTA_END_TICK,
                "volta",
            )?;
            Renderer::emit_text(
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
                MarkerGlyph::Text(text) => Renderer::emit_text(
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
                MarkerGlyph::Segno => Renderer::emit_segno(self.svg, x, baseline, self.fonts)?,
                MarkerGlyph::Fermata | MarkerGlyph::Staccato | MarkerGlyph::Marcato => {
                    Renderer::emit_touch(self.svg, &marker.glyph, x, baseline, self.fonts)?
                }
            }
        }

        for span in &layout.dynamic_spans {
            let start_x = self.origin_x + span.start_x;
            let end_x = self.origin_x + span.end_x;

            if let Some(label) = &span.label {
                Renderer::emit_text(
                    self.svg,
                    start_x,
                    y + span.baseline,
                    label,
                    "dynamic",
                    "start",
                )?;

                let dash_start = start_x + span.label_width + DYNAMIC_DASH_GAP;

                if dash_start < end_x {
                    Renderer::emit_line(
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

                Renderer::emit_line(
                    self.svg,
                    start_x,
                    left_top,
                    end_x,
                    right_top,
                    "dynamic-hairpin",
                )?;
                Renderer::emit_line(
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
                Renderer::emit_line(self.svg, x, top, x, bottom, "barline")?;
            }

            if pulse.section_end {
                let end_x = x + pulse.pulse_width;

                Renderer::emit_line(
                    self.svg,
                    end_x - DOUBLE_BARLINE_GAP,
                    top,
                    end_x - DOUBLE_BARLINE_GAP,
                    bottom,
                    "barline",
                )?;

                Renderer::emit_line(self.svg, end_x, top, end_x, bottom, "barline")?;
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
                } else if !voice_pulse.expanded {
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
    markers: Vec<PlacedMarker>,
    dynamic_spans: Vec<PlacedDynamicSpan>,
    voltas: Vec<PlacedVolta>,
    height: f32,
}

struct PlacedDynamicSpan {
    kind: DynamicSpanKind,
    start_x: f32,
    end_x: f32,
    center_y: f32,
    baseline: f32,
    label: Option<String>,
    label_width: f32,
}

struct PlacedVolta {
    start_x: f32,
    end_x: f32,
    text: String,
    text_x: f32,
    text_y: f32,
    line_y: f32,
}

#[derive(Clone)]
struct PlacedMarker {
    kind: MarkerKind,
    glyph: MarkerGlyph,
    width: f32,
    anchor: TextAnchor,
    x: f32,
    y: f32,
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
    Key,
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

fn marker_priority(kind: MarkerKind) -> usize {
    match kind {
        MarkerKind::Key => 0,
        MarkerKind::TimeSignature => 1,
        MarkerKind::Tempo | MarkerKind::TempoNumber => 2,
        MarkerKind::Jump => 3,
        MarkerKind::Mark => 4,
        MarkerKind::Dynamic => 5,
        MarkerKind::Touch => 6,
    }
}

fn touch_width(glyph: &MarkerGlyph, fonts: &Fonts) -> f32 {
    let size = fonts.marker.size() * TOUCH_SYMBOL_SCALE;

    match glyph {
        MarkerGlyph::Fermata => size * 1.7,
        MarkerGlyph::Staccato => size * 0.32,
        MarkerGlyph::Marcato => size * MARCATO_WIDTH_SCALE,
        _ => 0.0,
    }
}

fn marker_font(kind: MarkerKind, fonts: &Fonts) -> &FontMetrics {
    match kind {
        MarkerKind::Key | MarkerKind::Jump | MarkerKind::TempoNumber => &fonts.marker_bold,
        MarkerKind::TimeSignature => &fonts.marker_sans,
        MarkerKind::Touch => &fonts.marker,
        MarkerKind::Tempo | MarkerKind::Mark => &fonts.mark,
        MarkerKind::Dynamic => &fonts.dynamic,
    }
}

fn spread_markers(markers: &mut [PlacedMarker]) {
    markers.sort_by(|lhs, rhs| lhs.x.total_cmp(&rhs.x));

    let mut right_edge = f32::NEG_INFINITY;

    for marker in markers {
        let left = match marker.anchor {
            TextAnchor::Start => marker.x,
            TextAnchor::Middle => marker.x - marker.width / 2.0,
            TextAnchor::End => marker.x - marker.width,
        };

        let shift = (right_edge + MARKER_HORIZONTAL_GAP - left).max(0.0);

        marker.x += shift;
        right_edge = left + marker.width + shift;
    }
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

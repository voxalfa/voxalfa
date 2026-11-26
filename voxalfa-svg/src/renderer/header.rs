use voxalfa_core::ast::directives::{
    ElementStyle, HeaderCell, HeaderField, HeaderRow as HeaderRowDirective, HeaderTemplate,
    TemplatePart,
};

use crate::{
    fonts::{FontMetrics, Fonts},
    locale::Locale,
    model::{HeaderView, TextAnchor},
    options::RenderOptions,
};

pub fn layout_header(header: &HeaderView, fonts: &Fonts, options: &RenderOptions) -> HeaderLayout {
    let mut rows = HeaderRows::new(header, fonts, options);

    rows.push_rows();
    rows.into_layout()
}

#[derive(Clone)]
struct HeaderStyle<'a> {
    class: String,
    font: &'a FontMetrics,
    element: &'a ElementStyle,
}

struct HeaderStyles<'a> {
    fonts: &'a Fonts,
    options: &'a RenderOptions,
}

macro_rules! impl_header_styles {
    ($($name:literal => ($group:ident . $metric:ident, $style:ident)),* $(,)?) => {
        impl<'a> HeaderStyles<'a> {
            fn get(&self, name: &str) -> Option<HeaderStyle<'a>> {
                match name {
                    $(
                        $name => Some(HeaderStyle {
                            class: $name.to_string(),
                            font: &self.fonts.$group.$metric,
                            element: &self.options.elements.$style,
                        }),
                    )*
                    _ => self.custom(name),
                }
            }
        }
    };
}

impl_header_styles! {
    "title-text" => (text.title, title),
    "author-text" => (text.author, author),
    "composer-text" => (text.composer, composer),
    "header-text" => (text.header, header),
    "header-row" => (text.header_row, header_row),
    "key-text" => (text.key, key),
    "time-text" => (text.time, time),
    "meter-text" => (text.meter, meter),
    "note-text" => (text.note, note),
    "octave-text" => (text.octave, octave),
    "lyrics-text" => (text.lyric, lyrics),
    "divider-text" => (text.divider, divider),
    "footer-text" => (text.footer, footer),
    "key-label" => (label.key, key_label),
    "time-label" => (label.time, time_label),
    "tempo-label" => (label.tempo, tempo),
    "bpm-label" => (label.bpm, bpm),
    "jump-label" => (label.jump, jump),
    "mark-label" => (label.mark, mark),
    "dynamic-label" => (label.dynamic, dynamic),
    "volta-label" => (label.volta, volta),
}

impl<'a> HeaderStyles<'a> {
    fn new(fonts: &'a Fonts, options: &'a RenderOptions) -> Self {
        Self { fonts, options }
    }

    fn text(&self) -> HeaderStyle<'a> {
        self.get("header-text")
            .expect("header-text style is predefined")
    }

    fn custom(&self, name: &str) -> Option<HeaderStyle<'a>> {
        Some(HeaderStyle {
            class: name.to_string(),
            font: self.fonts.custom.get(name)?,
            element: self.options.elements.custom.get(name)?,
        })
    }
}

struct TextSegment<'a> {
    text: String,
    style: HeaderStyle<'a>,
    id: usize,
}

struct Cell<'a> {
    segments: Vec<TextSegment<'a>>,
    style: HeaderStyle<'a>,
    x: f32,
    anchor: TextAnchor,
}

struct HeaderRows<'a> {
    header: &'a HeaderView,
    fonts: &'a Fonts,
    options: &'a RenderOptions,
    rows: Vec<Row>,
    left: f32,
    right: f32,
}

impl<'a> HeaderRows<'a> {
    fn new(header: &'a HeaderView, fonts: &'a Fonts, options: &'a RenderOptions) -> Self {
        let left = options.page_margin.left;
        let right = options.page.width - options.page_margin.right;

        Self {
            header,
            fonts,
            options,
            rows: Vec::new(),
            left,
            right,
        }
    }

    fn style(&self, name: &str) -> HeaderStyle<'a> {
        let styles = HeaderStyles::new(self.fonts, self.options);

        styles.get(name).unwrap_or_else(|| styles.text())
    }

    fn push_rows(&mut self) {
        let rows = if self.header.rows.is_empty() {
            default_rows(self.header.is_bpm)
        } else {
            self.header.rows.clone()
        };

        for row in &rows {
            let style = self.row_style(row);
            let inset_left = style.map_or(0.0, ElementStyle::left);
            let inset_right = style.map_or(0.0, ElementStyle::right);
            let top = style.map_or(0.0, ElementStyle::top);
            let bottom = style.map_or(0.0, ElementStyle::bottom);
            let texts = self.row_texts(row, inset_left, inset_right);

            if !texts.is_empty() {
                self.push_row(texts, top, bottom);
            }
        }
    }

    fn row_style(&self, row: &HeaderRowDirective) -> Option<&'a ElementStyle> {
        let name = row.style.as_deref()?;
        HeaderStyles::new(self.fonts, self.options)
            .get(name)
            .map(|style| style.element)
    }

    fn row_texts(
        &self,
        row: &HeaderRowDirective,
        inset_left: f32,
        inset_right: f32,
    ) -> Vec<HeaderText> {
        let left = self.left + inset_left;
        let right = self.right - inset_right;
        let center = (left + right) / 2.0;

        let cells = [
            self.cell(row.left.as_ref(), left, TextAnchor::Start),
            self.cell(row.center.as_ref(), center, TextAnchor::Middle),
            self.cell(row.right.as_ref(), right, TextAnchor::End),
        ];

        let widths = self.cell_widths(&cells, (right - left).max(0.0));
        let mut texts = Vec::new();

        for (cell, width) in cells.into_iter().zip(widths) {
            if let Some(cell) = cell {
                texts.extend(self.wrap_cell(cell, width));
            }
        }

        texts
    }

    fn cell(&self, cell: Option<&HeaderCell>, x: f32, anchor: TextAnchor) -> Option<Cell<'a>> {
        let cell = cell?;
        let mut segments = Vec::new();

        for (index, template) in cell.templates.iter().enumerate() {
            let style = cell
                .styles
                .get(index)
                .and_then(|name| HeaderStyles::new(self.fonts, self.options).get(name))
                .unwrap_or_else(|| self.style("header-text"));

            for part in &template.parts {
                match part {
                    TemplatePart::Text(text) => {
                        if !text.is_empty() {
                            segments.push(TextSegment {
                                text: text.clone(),
                                style: style.clone(),
                                id: index,
                            });
                        }
                    }
                    TemplatePart::Field(field) => segments.push(TextSegment {
                        text: self.field_value(*field),
                        style: style.clone(),
                        id: index,
                    }),
                }
            }
        }

        if segments
            .iter()
            .all(|segment| segment.text.trim().is_empty())
        {
            return None;
        }

        let style = segments
            .first()
            .map(|segment| segment.style.clone())
            .unwrap_or_else(|| self.style("header-text"));

        Some(Cell {
            segments,
            style,
            x,
            anchor,
        })
    }

    fn cell_widths(&self, cells: &[Option<Cell<'a>>; 3], available: f32) -> [f32; 3] {
        let count = cells.iter().flatten().count();

        if count == 0 {
            return [0.0; 3];
        }

        let width = available / count as f32;

        cells
            .each_ref()
            .map(|cell| if cell.is_some() { width } else { 0.0 })
    }

    fn wrap_cell(&self, cell: Cell<'a>, width: f32) -> Vec<HeaderText> {
        let available = (width - cell.style.element.x_space()).max(1.0);
        let lines = self.wrap_segments(&cell.segments, available);
        let mut texts = Vec::new();

        for (line, segments) in lines.into_iter().enumerate() {
            if let [segment] = segments.as_slice() {
                texts.push(
                    HeaderText::new(
                        segment.text.clone(),
                        cell.x,
                        cell.anchor,
                        segment.style.clone(),
                    )
                    .at_line(line),
                );

                continue;
            }

            let width = self.line_width(&segments);

            let start = match cell.anchor {
                TextAnchor::Start => cell.x + cell.style.element.left(),
                TextAnchor::Middle => {
                    cell.x + (cell.style.element.left() - cell.style.element.right() - width) / 2.0
                }
                TextAnchor::End => cell.x - cell.style.element.right() - width,
            };

            let mut x = start;
            let mut previous: Option<(usize, HeaderStyle<'a>)> = None;

            for segment in segments {
                if let Some((index, style)) = &previous
                    && *index != segment.id
                {
                    x += style.element.gap_to(&segment.style.element);
                }

                let segment_width = segment.style.font.width(&segment.text);
                let index = segment.id;
                let style = segment.style.clone();

                texts.push(HeaderText::start(segment.text, x, segment.style, line));
                x += segment_width;
                previous = Some((index, style));
            }
        }

        texts
    }

    fn line_width(&self, segments: &[TextSegment<'a>]) -> f32 {
        let mut width = 0.0;
        let mut previous: Option<&TextSegment<'a>> = None;

        for segment in segments {
            if let Some(previous) = previous {
                width += self.segment_gap(previous, segment);
            }

            width += segment.style.font.width(&segment.text);
            previous = Some(segment);
        }

        width
    }

    fn segment_gap(&self, previous: &TextSegment<'a>, current: &TextSegment<'a>) -> f32 {
        if previous.id == current.id {
            0.0
        } else {
            previous.style.element.gap_to(&current.style.element)
        }
    }

    fn wrap_segments(&self, segments: &[TextSegment<'a>], width: f32) -> Vec<Vec<TextSegment<'a>>> {
        let mut lines = Vec::new();
        let mut line = Vec::new();
        let mut line_width = 0.0;

        for token in self.tokens(segments) {
            let token_width = token.style.font.width(&token.text);
            let gap = match line.last() {
                Some(previous) => self.segment_gap(previous, &token),
                None => 0.0,
            };

            if !line.is_empty() && line_width + gap + token_width > width {
                lines.push(self.merge(std::mem::take(&mut line)));
                line_width = token_width;
                line.push(token);
                continue;
            }

            line_width += gap + token_width;
            line.push(token);
        }

        if !line.is_empty() {
            lines.push(self.merge(line));
        }

        if lines.is_empty() {
            lines.push(Vec::new());
        }

        lines
    }

    fn merge(&self, segments: Vec<TextSegment<'a>>) -> Vec<TextSegment<'a>> {
        let mut merged: Vec<TextSegment<'a>> = Vec::new();

        for segment in segments {
            if let Some(last) = merged.last_mut()
                && last.id == segment.id
                && last.style.class == segment.style.class
            {
                last.text.push_str(&segment.text);
            } else {
                merged.push(segment);
            }
        }

        merged
    }

    fn tokens(&self, segments: &[TextSegment<'a>]) -> Vec<TextSegment<'a>> {
        let mut tokens = Vec::new();

        for segment in segments {
            let mut token = String::new();
            let mut space = None;

            for ch in segment.text.chars() {
                let is_space = ch.is_whitespace();

                if space.is_some_and(|space| space != is_space) {
                    tokens.push(TextSegment {
                        text: std::mem::take(&mut token),
                        style: segment.style.clone(),
                        id: segment.id,
                    });
                }

                space = Some(is_space);
                token.push(ch);
            }

            if !token.is_empty() {
                tokens.push(TextSegment {
                    text: token,
                    style: segment.style.clone(),
                    id: segment.id,
                });
            }
        }

        tokens
    }

    fn field_value(&self, field: HeaderField) -> String {
        match field {
            HeaderField::Title => self.header.title.clone().unwrap_or_default(),
            HeaderField::Author => self.header.authors.join(", "),
            HeaderField::Composer => self.header.composers.join(", "),
            HeaderField::Key => self.format_key(),
            HeaderField::Time => self.header.time.clone().unwrap_or_default(),
            HeaderField::Tempo => self.header.tempo.clone().unwrap_or_default(),
            HeaderField::Meter => self.header.meter.clone().unwrap_or_default(),
            HeaderField::Language => self.header.language.clone().unwrap_or_default(),
            HeaderField::Description => self.header.description.clone().unwrap_or_default(),
        }
    }

    fn format_key(&self) -> String {
        let locale = Locale::from_language(self.header.language.as_deref());

        self.header
            .key
            .as_ref()
            .map(|key| locale.key_label(key))
            .unwrap_or_default()
    }

    fn push_row(&mut self, mut texts: Vec<HeaderText>, top: f32, bottom: f32) {
        let baseline = texts
            .iter()
            .filter(|text| text.line == 0)
            .map(|text| text.baseline)
            .fold(0.0, f32::max);

        for text in texts.iter_mut() {
            text.baseline = baseline + text.line as f32 * text.line_height;
        }

        let height = texts
            .iter()
            .map(|text| text.baseline + text.bottom)
            .fold(0.0, f32::max);

        self.rows.push(Row {
            texts,
            height,
            top,
            bottom,
        });
    }

    fn into_layout(self) -> HeaderLayout {
        if self.rows.is_empty() {
            return HeaderLayout {
                lines: Vec::new(),
                height: 0.0,
            };
        }

        let mut lines = Vec::new();
        let mut y = 0.0;
        let last = self.rows.len().saturating_sub(1);

        for (index, row) in self.rows.into_iter().enumerate() {
            y += row.top;

            for text in row.texts {
                lines.push(HeaderLine {
                    text: text.text,
                    class: text.class,
                    x: text.x,
                    y: y + text.baseline,
                    anchor: text.anchor,
                });
            }

            y += row.height;

            if index != last {
                y += row.bottom;
            }
        }

        HeaderLayout { lines, height: y }
    }
}

#[derive(Debug)]
pub struct HeaderLine {
    pub text: String,
    pub class: String,
    pub x: f32,
    pub y: f32,
    pub anchor: TextAnchor,
}

#[derive(Debug)]
struct HeaderText {
    text: String,
    class: String,
    x: f32,
    baseline: f32,
    bottom: f32,
    line: usize,
    line_height: f32,
    anchor: TextAnchor,
}

impl HeaderText {
    fn new(text: impl Into<String>, x: f32, anchor: TextAnchor, style: HeaderStyle<'_>) -> Self {
        let line_height = style.font.line_height();
        let baseline = style.element.top() + line_height;

        Self {
            text: text.into(),
            class: style.class.clone(),
            x: anchor.align_x(x, style.element),
            baseline,
            bottom: style.element.bottom(),
            line: 0,
            line_height,
            anchor,
        }
    }

    fn start(text: impl Into<String>, x: f32, style: HeaderStyle<'_>, line: usize) -> Self {
        let line_height = style.font.line_height();

        Self {
            text: text.into(),
            class: style.class.clone(),
            x,
            baseline: style.element.top() + line_height,
            bottom: style.element.bottom(),
            line,
            line_height,
            anchor: TextAnchor::Start,
        }
    }

    fn at_line(mut self, line: usize) -> Self {
        self.line = line;
        self
    }
}

#[derive(Debug)]
struct Row {
    texts: Vec<HeaderText>,
    height: f32,
    top: f32,
    bottom: f32,
}

#[derive(Debug)]
pub struct HeaderLayout {
    pub lines: Vec<HeaderLine>,
    pub height: f32,
}

fn default_rows(is_bpm: bool) -> Vec<HeaderRowDirective> {
    let tempo = if is_bpm { "bpm-label" } else { "tempo-label" };

    vec![
        HeaderRowDirective {
            center: Some(cell_parts(&[(field(HeaderField::Title), "title-text")])),
            style: Some("header-row".to_string()),
            ..Default::default()
        },
        HeaderRowDirective {
            left: Some(cell_parts(&[(field(HeaderField::Author), "author-text")])),
            center: Some(cell_parts(&[(field(HeaderField::Meter), "meter-text")])),
            right: Some(cell_parts(&[(
                field(HeaderField::Composer),
                "composer-text",
            )])),
            style: Some("header-row".to_string()),
        },
        HeaderRowDirective {
            left: Some(cell_parts(&[
                (field(HeaderField::Key), "key-text"),
                (field(HeaderField::Time), "time-text"),
                (field(HeaderField::Tempo), tempo),
            ])),
            ..Default::default()
        },
    ]
}

fn cell_parts(parts: &[(HeaderTemplate, &str)]) -> HeaderCell {
    HeaderCell {
        templates: parts.iter().map(|(template, _)| template.clone()).collect(),
        styles: parts.iter().map(|(_, style)| style.to_string()).collect(),
    }
}

fn field(field: HeaderField) -> HeaderTemplate {
    HeaderTemplate {
        parts: vec![TemplatePart::Field(field)],
    }
}

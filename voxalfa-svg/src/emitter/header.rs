use voxalfa_core::ast::directives::{
    ElementStyle, HeaderCell, HeaderField, HeaderRow as HeaderRowDirective, HeaderTemplate,
    TemplatePart,
};

use crate::{
    error::Result,
    fonts::{FontMetrics, Fonts},
    locale::Locale,
    model::{HeaderView, TextAnchor},
    options::RenderOptions,
    renderer::constants::*,
};

use super::primitives::emit_text;

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

struct Run<'a> {
    text: String,
    style: HeaderStyle<'a>,
    run: usize,
}

struct Cell<'a> {
    runs: Vec<Run<'a>>,
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
        let mut runs = Vec::new();

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
                            runs.push(Run {
                                text: text.clone(),
                                style: style.clone(),
                                run: index,
                            });
                        }
                    }
                    TemplatePart::Field(field) => runs.push(Run {
                        text: self.field_value(*field),
                        style: style.clone(),
                        run: index,
                    }),
                }
            }
        }

        if runs.iter().all(|run| run.text.trim().is_empty()) {
            return None;
        }

        let style = runs
            .first()
            .map(|run| run.style.clone())
            .unwrap_or_else(|| self.style("header-text"));

        Some(Cell {
            runs,
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
        let available = (width - cell.style.element.left() - cell.style.element.right()).max(1.0);
        let lines = self.wrap_runs(&cell.runs, available);
        let mut texts = Vec::new();

        for (line, runs) in lines.into_iter().enumerate() {
            if runs.len() == 1 {
                let run = runs.into_iter().next().expect("one run");
                texts.push(HeaderText::new(run.text, cell.x, cell.anchor, run.style).at_line(line));
                continue;
            }

            let width = self.line_width(&runs);
            let start = match cell.anchor {
                TextAnchor::Start => cell.x + cell.style.element.left(),
                TextAnchor::Middle => {
                    cell.x + (cell.style.element.left() - cell.style.element.right() - width) / 2.0
                }
                TextAnchor::End => cell.x - cell.style.element.right() - width,
            };
            let mut x = start;
            let mut previous: Option<(usize, HeaderStyle<'a>)> = None;

            for run in runs {
                if let Some((index, style)) = &previous
                    && *index != run.run
                {
                    x += style.element.right() + run.style.element.left();
                }

                let run_width = run.style.font.width(&run.text);
                let index = run.run;
                let style = run.style.clone();

                texts.push(HeaderText::start(run.text, x, run.style, line));
                x += run_width;
                previous = Some((index, style));
            }
        }

        texts
    }

    fn line_width(&self, runs: &[Run<'a>]) -> f32 {
        let mut width = 0.0;
        let mut previous: Option<&Run<'a>> = None;

        for run in runs {
            if let Some(previous) = previous {
                width += self.run_gap(previous, run);
            }

            width += run.style.font.width(&run.text);
            previous = Some(run);
        }

        width
    }

    fn run_gap(&self, previous: &Run<'a>, current: &Run<'a>) -> f32 {
        if previous.run == current.run {
            0.0
        } else {
            previous.style.element.right() + current.style.element.left()
        }
    }

    fn wrap_runs(&self, runs: &[Run<'a>], width: f32) -> Vec<Vec<Run<'a>>> {
        let mut lines = Vec::new();
        let mut line = Vec::new();
        let mut line_width = 0.0;

        for token in self.tokens(runs) {
            let token_width = token.style.font.width(&token.text);
            let gap = if line.is_empty() {
                0.0
            } else {
                self.run_gap(line.last().expect("non-empty line"), &token)
            };

            if token.text.chars().all(char::is_whitespace) {
                if !line.is_empty() {
                    line_width += token_width;
                    line.push(token);
                }
                continue;
            }

            if !line.is_empty() && line_width + gap + token_width > width {
                while line
                    .last()
                    .is_some_and(|token: &Run<'a>| token.text.chars().all(char::is_whitespace))
                {
                    line.pop();
                }

                lines.push(self.merge(std::mem::take(&mut line)));
                line_width = 0.0;
            }

            line_width += gap + token_width;
            line.push(token);
        }

        while line
            .last()
            .is_some_and(|token| token.text.chars().all(char::is_whitespace))
        {
            line.pop();
        }

        if !line.is_empty() {
            lines.push(self.merge(line));
        }

        if lines.is_empty() {
            lines.push(Vec::new());
        }

        lines
    }

    fn merge(&self, runs: Vec<Run<'a>>) -> Vec<Run<'a>> {
        let mut merged: Vec<Run<'a>> = Vec::new();

        for run in runs {
            if let Some(last) = merged.last_mut()
                && last.run == run.run
                && last.style.class == run.style.class
            {
                last.text.push_str(&run.text);
                continue;
            }

            merged.push(run);
        }

        merged
    }

    fn tokens(&self, runs: &[Run<'a>]) -> Vec<Run<'a>> {
        let mut tokens = Vec::new();

        for run in runs {
            let mut token = String::new();
            let mut space = None;

            for ch in run.text.chars() {
                let is_space = ch.is_whitespace();

                if space.is_some_and(|space| space != is_space) {
                    tokens.push(Run {
                        text: std::mem::take(&mut token),
                        style: run.style.clone(),
                        run: run.run,
                    });
                }

                space = Some(is_space);
                token.push(ch);
            }

            if !token.is_empty() {
                tokens.push(Run {
                    text: token,
                    style: run.style.clone(),
                    run: run.run,
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
            HeaderField::Key => {
                let locale = Locale::from_language(self.header.language.as_deref());

                self.header
                    .key
                    .as_ref()
                    .map(|key| locale.key_label(key))
                    .unwrap_or_default()
            }
            HeaderField::Time => self.header.time.clone().unwrap_or_default(),
            HeaderField::Tempo => self.header.tempo.clone().unwrap_or_default(),
            HeaderField::Meter => self.header.meter.clone().unwrap_or_default(),
            HeaderField::Language => self.header.language.clone().unwrap_or_default(),
            HeaderField::Description => self.header.description.clone().unwrap_or_default(),
        }
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
        let bottom_padding = self.style("meter-text").font.line_height() * HEADER_BOTTOM_PADDING;

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

        HeaderLayout {
            lines,
            height: y + bottom_padding,
        }
    }
}

pub fn emit_header(
    svg: &mut String,
    header: &HeaderLayout,
    options: &RenderOptions,
    offset_y: f32,
) -> Result<()> {
    for line in &header.lines {
        emit_text(
            svg,
            line.x,
            options.page_margin.top + offset_y + line.y,
            &line.text,
            &line.class,
            line.anchor.as_str(),
        )?;
    }

    Ok(())
}

#[derive(Debug)]
struct HeaderLine {
    text: String,
    class: String,
    x: f32,
    y: f32,
    anchor: TextAnchor,
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
    lines: Vec<HeaderLine>,
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

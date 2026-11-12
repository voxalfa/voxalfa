use voxalfa_core::ast::directives::ElementStyle;

use crate::{
    error::Result,
    fonts::{FontMetrics, Fonts},
    locale::Locale,
    model::HeaderView,
    options::RenderOptions,
};

use super::{
    constants::*,
    primitives::{TextAnchor, emit_text},
};

pub fn layout_header(header: &HeaderView, fonts: &Fonts, options: &RenderOptions) -> HeaderLayout {
    let mut rows = HeaderRows::new(header, fonts, options);

    rows.push_title();
    rows.push_credits();
    rows.push_meta();
    rows.into_layout()
}

struct HeaderRows<'a> {
    header: &'a HeaderView,
    fonts: &'a Fonts,
    options: &'a RenderOptions,
    rows: Vec<HeaderRow>,
    left: f32,
    right: f32,
    center: f32,
}

impl<'a> HeaderRows<'a> {
    fn new(header: &'a HeaderView, fonts: &'a Fonts, options: &'a RenderOptions) -> Self {
        let left = options.page_margin.left;
        let center = left + options.printable_width() / 2.0;
        let right = options.page.width - options.page_margin.right;

        Self {
            header,
            fonts,
            options,
            rows: Vec::new(),
            left,
            right,
            center,
        }
    }

    fn push_title(&mut self) {
        let Some(title) = &self.header.title else {
            return;
        };

        let style = &self.options.elements.text.title;
        let width = (self.options.printable_width() - style.left() - style.right()).max(1.0);

        for line in wrap_text(title, width, &self.fonts.text.title) {
            let text = header_text(
                line,
                HeaderKind::Title,
                self.center,
                TextAnchor::Middle,
                &self.fonts.text.title,
                style,
                TITLE_LINE_HEIGHT_FACTOR,
            );

            self.push_row(vec![text], 0.0);
        }
    }

    fn push_credits(&mut self) {
        let authors = self.header.authors.join(", ");
        let composers = self.header.composers.join(", ");

        let styles = &self.options.elements;
        let mut texts = Vec::new();

        // move the author to the metadata row if composer is empty
        if composers.is_empty() {
            return;
        }

        texts.push(header_text(
            composers,
            HeaderKind::Composer,
            self.right,
            TextAnchor::End,
            &self.fonts.text.composer,
            &styles.text.composer,
            1.0,
        ));

        if !authors.is_empty() {
            texts.push(header_text(
                authors,
                HeaderKind::Author,
                self.left,
                TextAnchor::Start,
                &self.fonts.text.author,
                &styles.text.author,
                1.0,
            ));
        }

        let gap_before = if self.rows.is_empty() {
            0.0
        } else {
            self.fonts.text.title.line_height() * HEADER_TITLE_BOTTOM_GAP
        };

        self.push_row(texts, gap_before);
    }

    fn push_meta(&mut self) {
        let styles = &self.options.elements;
        let locale = Locale::from_language(self.header.language.as_deref());
        let key = self.header.key.as_ref().map(|key| locale.key_label(key));
        let (time_x, tempo_x) = self.meta_x_positions(key.as_deref(), self.header.time.as_deref());
        let author_on_meta = self.header.composers.is_empty() && !self.header.authors.is_empty();

        if key.is_none()
            && self.header.meter.is_none()
            && self.header.time.is_none()
            && self.header.tempo.is_none()
            && !author_on_meta
        {
            return;
        }

        let mut texts = Vec::new();

        if author_on_meta {
            texts.push(header_text(
                self.header.authors.join(", "),
                HeaderKind::Author,
                self.right,
                TextAnchor::End,
                &self.fonts.text.author,
                &styles.text.author,
                1.0,
            ));
        }

        if let Some(text) = key {
            texts.push(header_text(
                text,
                HeaderKind::Key,
                self.left,
                TextAnchor::Start,
                &self.fonts.text.key,
                &styles.text.key,
                1.0,
            ));
        }

        if let Some(time) = &self.header.time {
            texts.push(header_text(
                time.clone(),
                HeaderKind::Time,
                time_x,
                TextAnchor::Start,
                &self.fonts.text.time,
                &styles.text.time,
                1.0,
            ));
        }

        if let Some(tempo) = &self.header.tempo {
            let (kind, font, style) = if self.header.is_bpm {
                (HeaderKind::Bpm, &self.fonts.label.bpm, &styles.label.bpm)
            } else {
                (
                    HeaderKind::Tempo,
                    &self.fonts.label.tempo,
                    &styles.label.tempo,
                )
            };

            texts.push(header_text(
                tempo.clone(),
                kind,
                tempo_x,
                TextAnchor::Start,
                font,
                style,
                1.0,
            ));
        }

        if let Some(meter) = &self.header.meter {
            texts.push(header_text(
                meter.clone(),
                HeaderKind::Meter,
                self.center,
                TextAnchor::Middle,
                &self.fonts.text.meter,
                &styles.text.meter,
                1.0,
            ));
        }

        let gap_before = if self.rows.is_empty() {
            0.0
        } else {
            self.fonts.text.meter.line_height() * HEADER_GROUP_GAP
        };

        self.push_row(texts, gap_before);
    }

    fn meta_x_positions(&self, key: Option<&str>, time: Option<&str>) -> (f32, f32) {
        let styles = &self.options.elements;

        let time_x = key.map_or(self.left, |text| {
            self.left
                + styles.text.key.left()
                + self.fonts.text.key.width(text)
                + styles.text.key.right()
                + self.fonts.text.key.size() * HEADER_KEY_RIGHT_MARGIN
        });

        let tempo_x = time_x
            + time.map_or(0.0, |time| {
                self.fonts.text.time.width(time)
                    + self.fonts.text.time.size() * HEADER_TIME_RIGHT_MARGIN
            });

        (time_x, tempo_x)
    }

    fn push_row(&mut self, mut texts: Vec<(HeaderText, f32)>, gap_before: f32) {
        let baseline = texts
            .iter()
            .map(|(text, _)| text.baseline)
            .fold(0.0, f32::max);

        let height = texts
            .iter()
            .map(|(text, text_height)| baseline + (*text_height - text.baseline))
            .fold(0.0, f32::max);

        for (text, _) in &mut texts {
            text.baseline = baseline;
        }

        self.rows.push(HeaderRow {
            texts: texts.into_iter().map(|(text, _)| text).collect(),
            height,
            gap_before,
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

        for row in self.rows {
            y += row.gap_before;

            for text in row.texts {
                lines.push(HeaderLine {
                    text: text.text,
                    kind: text.kind,
                    x: text.x,
                    y: y + text.baseline,
                    anchor: text.anchor,
                });
            }

            y += row.height;
        }

        HeaderLayout {
            lines,
            height: y + self.fonts.text.meter.line_height() * HEADER_BOTTOM_PADDING,
        }
    }
}

pub fn emit_header(svg: &mut String, header: &HeaderLayout, options: &RenderOptions) -> Result<()> {
    for line in &header.lines {
        let class = match line.kind {
            HeaderKind::Title => "title-text",
            HeaderKind::Author => "author-text",
            HeaderKind::Composer => "composer-text",
            HeaderKind::Key => "key-text",
            HeaderKind::Time => "time-text",
            HeaderKind::Tempo => "tempo-label",
            HeaderKind::Bpm => "bpm-label",
            HeaderKind::Meter => "meter-text",
        };

        emit_text(
            svg,
            line.x,
            options.page_margin.top + line.y,
            &line.text,
            class,
            line.anchor.as_str(),
        )?;
    }

    Ok(())
}

#[derive(Debug, Clone, Copy)]
enum HeaderKind {
    Title,
    Author,
    Composer,
    Key,
    Time,
    Tempo,
    Bpm,
    Meter,
}

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
    baseline: f32,
    anchor: TextAnchor,
}

struct HeaderRow {
    texts: Vec<HeaderText>,
    height: f32,
    gap_before: f32,
}

pub struct HeaderLayout {
    lines: Vec<HeaderLine>,
    pub height: f32,
}

fn header_text(
    text: String,
    kind: HeaderKind,
    x: f32,
    anchor: TextAnchor,
    font: &FontMetrics,
    style: &ElementStyle,
    line_factor: f32,
) -> (HeaderText, f32) {
    let baseline = style.top() + font.line_height() * line_factor;
    let height = baseline + style.bottom();

    (
        HeaderText {
            text,
            kind,
            x: anchor_x(x, anchor, style),
            baseline,
            anchor,
        },
        height,
    )
}

fn anchor_x(x: f32, anchor: TextAnchor, style: &ElementStyle) -> f32 {
    match anchor {
        TextAnchor::Start => x + style.left(),
        TextAnchor::Middle => x + (style.left() - style.right()) / 2.0,
        TextAnchor::End => x - style.right(),
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

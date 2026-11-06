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
    let left = options.page_margin.left;

    let right = options.page.width
        - options.page_margin.right
        - fonts.text.meter.size() * HEADER_RIGHT_INSET;

    let center = left + options.printable_width() / 2.0;
    let styles = &options.elements;

    let mut rows = Vec::new();

    if let Some(title) = &header.title {
        let style = &styles.text.title;
        let width = (options.printable_width() - style.left() - style.right()).max(1.0);

        for line in wrap_text(title, width, &fonts.text.title) {
            let text = header_text(
                line,
                HeaderKind::Title,
                center,
                TextAnchor::Middle,
                &fonts.text.title,
                style,
                TITLE_LINE_HEIGHT_FACTOR,
            );

            push_row(&mut rows, vec![text], 0.0);
        }
    }

    let authors = header.authors.join(", ");
    let composers = header.composers.join(", ");

    if !authors.is_empty() || !composers.is_empty() {
        let mut texts = Vec::new();

        if !authors.is_empty() {
            texts.push(header_text(
                authors,
                HeaderKind::Author,
                left,
                TextAnchor::Start,
                &fonts.text.author,
                &styles.text.author,
                1.0,
            ));
        }

        if !composers.is_empty() {
            texts.push(header_text(
                composers,
                HeaderKind::Composer,
                right,
                TextAnchor::End,
                &fonts.text.composer,
                &styles.text.composer,
                1.0,
            ));
        }

        let gap_before = if rows.is_empty() {
            0.0
        } else {
            fonts.text.title.line_height() * HEADER_TITLE_BOTTOM_GAP
        };

        push_row(&mut rows, texts, gap_before);
    }

    let locale = Locale::from_language(header.language.as_deref());
    let key = header.key.as_ref().map(|key| locale.key_label(key));

    let time_x = key
        .as_ref()
        .map(|text| {
            left + styles.text.key.left()
                + fonts.text.key.width(text)
                + styles.text.key.right()
                + fonts.text.key.size() * HEADER_KEY_RIGHT_MARGIN
        })
        .unwrap_or(left);

    if key.is_some() || header.meter.is_some() || header.time.is_some() {
        let mut texts = Vec::new();

        if let Some(text) = key {
            texts.push(header_text(
                text,
                HeaderKind::Key,
                left,
                TextAnchor::Start,
                &fonts.text.key,
                &styles.text.key,
                1.0,
            ));
        }

        if let Some(time) = &header.time {
            texts.push(header_text(
                time.clone(),
                HeaderKind::Time,
                time_x,
                TextAnchor::Start,
                &fonts.text.time,
                &styles.text.time,
                1.0,
            ));
        }

        if let Some(meter) = &header.meter {
            texts.push(header_text(
                meter.clone(),
                HeaderKind::Meter,
                center,
                TextAnchor::Middle,
                &fonts.text.meter,
                &styles.text.meter,
                1.0,
            ));
        }

        let gap_before = if rows.is_empty() {
            0.0
        } else {
            fonts.text.meter.line_height() * HEADER_GROUP_GAP
        };

        push_row(&mut rows, texts, gap_before);
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
        height: y + fonts.text.meter.line_height() * HEADER_BOTTOM_PADDING,
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

fn push_row(rows: &mut Vec<HeaderRow>, mut texts: Vec<(HeaderText, f32)>, gap_before: f32) {
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

    rows.push(HeaderRow {
        texts: texts.into_iter().map(|(text, _)| text).collect(),
        height,
        gap_before,
    });
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

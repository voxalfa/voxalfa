use crate::{
    error::Result,
    fonts::{FontMetrics, Fonts},
    layout::*,
    locale::Locale,
    model::HeaderView,
    options::RenderOptions,
};

use super::primitives::{TextAnchor, emit_text};

pub fn layout_header(header: &HeaderView, fonts: &Fonts, options: &RenderOptions) -> HeaderLayout {
    let left = options.min_margin;
    let right = left + options.printable_width() - HEADER_RIGHT_INSET;
    let center = options.page.width / 2.0;
    let mut rows = Vec::new();

    if let Some(title) = &header.title {
        for line in wrap_text(title, options.printable_width(), &fonts.title) {
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

pub fn emit_header(svg: &mut String, header: &HeaderLayout, options: &RenderOptions) -> Result<()> {
    for line in &header.lines {
        let class = match line.kind {
            HeaderKind::Title => "title",
            HeaderKind::Name => "name",
            HeaderKind::Key => "key",
            HeaderKind::Meta => "meta",
        };

        emit_text(
            svg,
            line.x,
            options.min_margin + line.y,
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
    Name,
    Key,
    Meta,
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

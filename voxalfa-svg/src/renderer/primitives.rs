use std::fmt::Write;

use crate::{
    error::Result,
    fonts::Fonts,
    layout::*,
    model::MarkerGlyph,
    symbols::{
        FERMATA_GROUP, FERMATA_HEIGHT, FERMATA_WIDTH, MARCATO_HEIGHT, MARCATO_PATH, MARCATO_WIDTH,
        SEGNO_HEIGHT_SCALE, SEGNO_PATH, SEGNO_PATH_TRANSFORM, STACCATO_PATH, STACCATO_SIZE,
    },
};

use super::geometry::segno_width;

pub fn emit_text(
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

pub fn emit_line(svg: &mut String, x1: f32, y1: f32, x2: f32, y2: f32, class: &str) -> Result<()> {
    writeln!(
        svg,
        r#"<line x1="{x1:.2}" y1="{y1:.2}" x2="{x2:.2}" y2="{y2:.2}" class="{class}"/>"#,
    )?;

    Ok(())
}

pub fn emit_segno(svg: &mut String, x: f32, baseline: f32, fonts: &Fonts) -> Result<()> {
    let line_height = fonts.marker.line_height();
    let height = line_height * SEGNO_HEIGHT_SCALE;
    let scale = height / 3.0;
    let left = x - segno_width(fonts) / 2.0;
    let top = baseline - fonts.marker.ascent() + (line_height - height) / 2.0;

    writeln!(
        svg,
        r##"<g class="marker marker-segno" transform="translate({left:.2} {top:.2}) scale({scale:.4})"><path transform="{SEGNO_PATH_TRANSFORM}" d="{SEGNO_PATH}" fill="#333333"/></g>"##,
    )?;

    Ok(())
}

pub fn emit_touch(
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

#[derive(Debug, Clone, Copy)]
pub enum TextAnchor {
    Start,
    Middle,
    End,
}

impl TextAnchor {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Start => "start",
            Self::Middle => "middle",
            Self::End => "end",
        }
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

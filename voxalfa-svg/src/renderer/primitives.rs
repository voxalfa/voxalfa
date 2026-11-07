use std::fmt::Write;

use crate::{
    error::Result,
    fonts::Fonts,
    model::MarkerGlyph,
    symbols::{
        FERMATA_GROUP, FERMATA_HEIGHT, FERMATA_WIDTH, MARCATO_HEIGHT, MARCATO_PATH, MARCATO_WIDTH,
        SEGNO_HEIGHT_SCALE, SEGNO_PATH, SEGNO_PATH_TRANSFORM, STACCATO_PATH, STACCATO_SIZE,
    },
};

use super::{constants::*, geometry::segno_width};

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

#[derive(Debug, Clone, Copy)]
pub enum BraceSide {
    Left,
    Right,
}

pub fn emit_brace(
    svg: &mut String,
    center_x: f32,
    top: f32,
    bottom: f32,
    half_width: f32,
    side: BraceSide,
) -> Result<()> {
    let height = (bottom - top).max(0.0);
    let middle = top + height * 0.5;
    let x = center_x;
    let w = half_width;
    let hook = (height * 0.08).min(w * 1.5).max(0.5);
    let cusp = hook.min((height * 0.5 - hook).max(0.0));

    let dir = match side {
        BraceSide::Left => 1.0,
        BraceSide::Right => -1.0,
    };

    let tip = x + dir * w;
    let cusp_x = x - dir * w;
    let top_hook = top + hook;
    let upper_cusp = middle - cusp;
    let lower_cusp = middle + cusp;
    let bottom_hook = bottom - hook;

    let path = format!(
        "M {tip:.2} {top:.2} \
         Q {x:.2} {top:.2} {x:.2} {top_hook:.2} \
         L {x:.2} {upper_cusp:.2} \
         Q {x:.2} {middle:.2} {cusp_x:.2} {middle:.2} \
         Q {x:.2} {middle:.2} {x:.2} {lower_cusp:.2} \
         L {x:.2} {bottom_hook:.2} \
         Q {x:.2} {bottom:.2} {tip:.2} {bottom:.2}"
    );

    writeln!(svg, r#"<path d="{path}" class="brace"/>"#)?;

    Ok(())
}

pub fn emit_segno(svg: &mut String, x: f32, baseline: f32, fonts: &Fonts) -> Result<()> {
    let line_height = fonts.label.mark.line_height();
    let height = line_height * SEGNO_HEIGHT_SCALE;
    let scale = height / 3.0;
    let left = x - segno_width(fonts) / 2.0;
    let top = baseline - fonts.label.mark.ascent() + (line_height - height) / 2.0;

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
    let size = fonts.touch.size() * TOUCH_SYMBOL_SCALE;
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

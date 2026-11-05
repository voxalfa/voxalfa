use std::fmt::Write;

use base64::{Engine, engine::general_purpose::STANDARD};

use crate::{
    error::Result,
    fonts::{FontMetrics, Fonts},
};

use super::constants::*;

pub fn emit_defs(svg: &mut String, fonts: &Fonts, encode_font: bool) -> Result<()> {
    writeln!(svg, "<defs>\n  <style>")?;

    writeln!(
        svg,
        "    svg {{\n      background: #ffffff;\n    }}\n\n    text {{\n      fill: #1a1a1a;\n    }}"
    )?;

    emit_font_faces(svg, fonts, encode_font)?;
    emit_font_rules(svg, fonts)?;
    emit_symbol_styles(svg, fonts)?;

    Ok(())
}

fn emit_font_faces(svg: &mut String, fonts: &Fonts, encode_font: bool) -> Result<()> {
    if encode_font {
        for face in &fonts.faces {
            let style = if face.italic { "italic" } else { "normal" };
            let data = STANDARD.encode(face.data.as_ref());

            writeln!(
                svg,
                "    @font-face {{\n      font-family: '{}';\n      src: url('data:font/ttf;base64,{data}') format('truetype');\n      font-weight: {};\n      font-style: {style};\n    }}\n",
                face.family, face.weight,
            )?;
        }
    }

    Ok(())
}

fn emit_font_rules(svg: &mut String, fonts: &Fonts) -> Result<()> {
    font_rule(
        svg,
        ".note-text",
        &fonts.text.note,
        "bold",
        "normal",
        "monospace",
        None,
    )?;

    font_rule(
        svg,
        ".divider-text",
        &fonts.text.divider,
        "normal",
        "normal",
        "monospace",
        Some("#333333"),
    )?;

    font_rule(
        svg,
        ".octave-text",
        &fonts.text.octave,
        "bold",
        "normal",
        "monospace",
        None,
    )?;

    font_rule(
        svg,
        ".lyrics-text",
        &fonts.text.lyric,
        "normal",
        "normal",
        "sans-serif",
        None,
    )?;

    font_rule(
        svg,
        ".title-text",
        &fonts.text.title,
        "bold",
        "normal",
        "sans-serif",
        None,
    )?;

    font_rule(
        svg,
        ".author-text",
        &fonts.text.author,
        "normal",
        "normal",
        "sans-serif",
        None,
    )?;

    font_rule(
        svg,
        ".composer-text",
        &fonts.text.composer,
        "normal",
        "normal",
        "sans-serif",
        None,
    )?;

    font_rule(
        svg,
        ".key-text",
        &fonts.text.key,
        "bold",
        "normal",
        "'Noto Sans Symbols', sans-serif",
        None,
    )?;

    font_rule(
        svg,
        ".time-text",
        &fonts.text.time,
        "normal",
        "normal",
        "sans-serif",
        Some("#444444"),
    )?;

    font_rule(
        svg,
        ".meter-text",
        &fonts.text.meter,
        "normal",
        "normal",
        "sans-serif",
        Some("#444444"),
    )?;

    font_rule(
        svg,
        ".key-label",
        &fonts.label.key,
        "bold",
        "normal",
        "'Noto Sans Symbols', sans-serif",
        Some("#333333"),
    )?;

    font_rule(
        svg,
        ".jump-label",
        &fonts.label.jump,
        "bold",
        "normal",
        "sans-serif",
        Some("#333333"),
    )?;

    font_rule(
        svg,
        ".tempo-label",
        &fonts.label.tempo,
        "normal",
        "italic",
        "serif",
        Some("#333333"),
    )?;

    font_rule(
        svg,
        ".mark-label",
        &fonts.label.mark,
        "normal",
        "italic",
        "serif",
        Some("#333333"),
    )?;

    font_rule(
        svg,
        ".dynamic-label",
        &fonts.label.dynamic,
        "bold",
        "italic",
        "serif",
        None,
    )?;

    font_rule(
        svg,
        ".time-label",
        &fonts.label.time,
        "normal",
        "normal",
        "sans-serif",
        Some("#333333"),
    )?;

    font_rule(
        svg,
        ".volta-label",
        &fonts.label.volta,
        "bold",
        "normal",
        "sans-serif",
        Some("#333333"),
    )?;

    font_rule(
        svg,
        ".page-number-text",
        &fonts.text.page_number,
        "normal",
        "normal",
        "sans-serif",
        Some("#777777"),
    )?;

    Ok(())
}

fn emit_symbol_styles(svg: &mut String, fonts: &Fonts) -> Result<()> {
    writeln!(
        svg,
        r#"
    .volta {{
      stroke: #222222;
      stroke-width: {barline};
      stroke-linecap: square;
    }}

    .barline {{
      stroke: #222222;
      stroke-width: {barline};
      stroke-linecap: square;
    }}

    .medium-accent {{
      stroke: #333333;
      stroke-width: {accent};
      stroke-linecap: square;
    }}

    .underline {{
      stroke: #222222;
      stroke-width: {underline};
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
        barline = fonts.text.note.size() * BARLINE_STROKE_WIDTH,
        accent = fonts.text.note.size() * ACCENT_STROKE_WIDTH,
        underline = fonts.text.note.size() * UNDERLINE_STROKE_WIDTH,
    )?;

    Ok(())
}

fn font_rule(
    svg: &mut String,
    selector: &str,
    font: &FontMetrics,
    weight: &str,
    style: &str,
    fallback: &str,
    fill: Option<&str>,
) -> Result<()> {
    writeln!(
        svg,
        "    {selector} {{\n      font-family: '{}', {fallback};\n      font-size: {:.2}px;\n      font-weight: {weight};\n      font-style: {style};",
        font.family(),
        font.size(),
    )?;

    if let Some(fill) = fill {
        writeln!(svg, "      fill: {fill};")?;
    }

    writeln!(svg, "    }}\n")?;

    Ok(())
}

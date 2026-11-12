use std::fmt::Write;

use base64::{Engine, engine::general_purpose::STANDARD};
use voxalfa_core::ast::directives::{ElementStyle, FontStretch};

use crate::{
    error::Result,
    fonts::{FontMetrics, Fonts},
    options::RenderOptions,
};

use super::constants::*;

pub fn emit_defs(svg: &mut String, fonts: &Fonts, options: &RenderOptions) -> Result<()> {
    writeln!(svg, "<defs>\n  <style>")?;

    writeln!(
        svg,
        "    svg {{\n      background: #ffffff;\n    }}\n\n    text {{\n      fill: #1a1a1a;\n    }}"
    )?;

    emit_font_faces(svg, fonts, options.encode_font)?;
    emit_font_rules(svg, fonts, options)?;
    emit_symbol_styles(svg, fonts)?;

    Ok(())
}

fn emit_font_faces(svg: &mut String, fonts: &Fonts, encode_font: bool) -> Result<()> {
    if encode_font {
        for face in &fonts.faces {
            let style = if face.italic { "italic" } else { "normal" };
            let data = STANDARD.encode(face.data.as_ref());

            writeln!(svg, "    @font-face {{")?;
            writeln!(svg, "      font-family: '{}';", face.family)?;
            writeln!(
                svg,
                "      src: url('data:font/ttf;base64,{data}') format('truetype');"
            )?;
            writeln!(svg, "      font-weight: {};", face.weight)?;

            if let Some(stretch) = stretch_css(face.stretch) {
                writeln!(svg, "      font-stretch: {stretch};")?;
            }

            writeln!(svg, "      font-style: {style};")?;
            writeln!(svg, "    }}")?;
        }
    }

    Ok(())
}

fn emit_font_rules(svg: &mut String, fonts: &Fonts, options: &RenderOptions) -> Result<()> {
    let text = &fonts.text;
    let label = &fonts.label;
    let styles = &options.elements;
    let sans = "sans-serif";
    let mono = "monospace";
    let symbols = "'Noto Sans Symbols', sans-serif";

    let rules = [
        (".note-text", &text.note, mono, &styles.note),
        (".divider-text", &text.divider, sans, &styles.divider),
        (".octave-text", &text.octave, mono, &styles.octave),
        (".lyrics-text", &text.lyric, sans, &styles.lyrics),
        (".title-text", &text.title, sans, &styles.title),
        (".author-text", &text.author, sans, &styles.author),
        (".composer-text", &text.composer, sans, &styles.composer),
        (".key-text", &text.key, symbols, &styles.key),
        (".time-text", &text.time, sans, &styles.time),
        (".meter-text", &text.meter, sans, &styles.meter),
        (".key-label", &label.key, symbols, &styles.key_label),
        (".jump-label", &label.jump, sans, &styles.jump),
        (".tempo-label", &label.tempo, "serif", &styles.tempo),
        (".bpm-label", &label.bpm, "serif", &styles.bpm),
        (".mark-label", &label.mark, "serif", &styles.mark),
        (".dynamic-label", &label.dynamic, "serif", &styles.dynamic),
        (".time-label", &label.time, sans, &styles.time_label),
        (".volta-label", &label.volta, sans, &styles.volta),
        (".footer-text", &text.footer, sans, &styles.footer),
    ];

    for (selector, font, fallback, style) in rules {
        font_rule(svg, selector, font, style, fallback)?;
    }

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

    .brace {{
      fill: none;
      stroke: #222222;
      stroke-width: {brace};
      stroke-linecap: round;
      stroke-linejoin: round;
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
        brace = fonts.text.note.size() * BARLINE_STROKE_WIDTH,
        accent = fonts.text.note.size() * ACCENT_STROKE_WIDTH,
        underline = fonts.text.note.size() * UNDERLINE_STROKE_WIDTH,
    )?;

    Ok(())
}

fn font_rule(
    svg: &mut String,
    selector: &str,
    font: &FontMetrics,
    element: &ElementStyle,
    fallback: &str,
) -> Result<()> {
    let style = if font.italic() { "italic" } else { "normal" };
    let fill = element.color.as_deref().unwrap_or("black");

    writeln!(svg, "    {selector} {{")?;
    writeln!(svg, "      font-family: '{}', {fallback};", font.family())?;
    writeln!(svg, "      font-size: {:.2}px;", font.size())?;
    writeln!(svg, "      font-weight: {};", font.weight())?;

    if let Some(stretch) = stretch_css(font.stretch()) {
        writeln!(svg, "      font-stretch: {stretch};")?;
    }

    writeln!(svg, "      font-style: {style};")?;
    writeln!(svg, "      fill: {fill};")?;
    writeln!(svg, "    }}")?;

    Ok(())
}

fn stretch_css(stretch: FontStretch) -> Option<&'static str> {
    match stretch {
        FontStretch::UltraCondensed => Some("ultra-condensed"),
        FontStretch::ExtraCondensed => Some("extra-condensed"),
        FontStretch::Condensed => Some("condensed"),
        FontStretch::SemiCondensed => Some("semi-condensed"),
        FontStretch::Normal => None,
        FontStretch::SemiExpanded => Some("semi-expanded"),
        FontStretch::Expanded => Some("expanded"),
        FontStretch::ExtraExpanded => Some("extra-expanded"),
        FontStretch::UltraExpanded => Some("ultra-expanded"),
    }
}

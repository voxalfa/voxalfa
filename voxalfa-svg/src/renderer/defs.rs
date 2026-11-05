use std::fmt::Write;

use base64::{Engine, engine::general_purpose::STANDARD};

use crate::{
    error::Result,
    fonts::{
        LYRIC_FAMILY, LYRIC_FONT, SERIF_BOLD_ITALIC_FONT, SERIF_FAMILY, SERIF_FONT,
        SERIF_ITALIC_FONT, SOLFA_FAMILY, SOLFA_FONT, TEXT_BOLD_FONT, TEXT_FAMILY, TEXT_FONT,
    },
    layout::*,
    options::RenderOptions,
};

pub fn emit_defs(svg: &mut String, options: &RenderOptions) -> Result<()> {
    let solfa = STANDARD.encode(SOLFA_FONT);
    let text = STANDARD.encode(TEXT_FONT);
    let text_bold = STANDARD.encode(TEXT_BOLD_FONT);
    let serif = STANDARD.encode(SERIF_FONT);
    let serif_italic = STANDARD.encode(SERIF_ITALIC_FONT);
    let serif_bold_italic = STANDARD.encode(SERIF_BOLD_ITALIC_FONT);
    let lyric = STANDARD.encode(LYRIC_FONT);

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

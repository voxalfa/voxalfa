use base64::{Engine, engine::general_purpose::STANDARD};

use crate::fonts::{LYRIC_FONT, LYRIC_FONT_SIZE, SOLFA_FONT, SOLFA_FONT_SIZE};

pub fn build_svg_defs() -> String {
    let solfa_font_b64 = STANDARD.encode(SOLFA_FONT);
    let lyrics_font_b64 = STANDARD.encode(LYRIC_FONT);

    format!(
        r#"<defs>
  <style>
    @font-face {{
      font-family: 'FiraSans-Solfa';
      src: url('data:font/ttf;charset=utf-8;base64,{solfa_font_b64}') format('truetype');
      font-weight: normal;
      font-style: normal;
    }}

    @font-face {{
      font-family: 'NotoSans-Lyrics';
      src: url('data:font/ttf;charset=utf-8;base64,{lyrics_font_b64}') format('truetype');
    }}

    .lyric {{
      font-family: 'NotoSans-Lyrics', sans-serif;
      font-size: {LYRIC_FONT_SIZE}px;
      fill: currentColor;
    }}

    .solfa {{
      font-family: 'FiraSans-Solfa', sans-serif;
      font-size: {SOLFA_FONT_SIZE}px;
      fill: #1a1a1a;
    }}
  </style>
</defs>"#
    )
}

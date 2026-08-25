use base64::{Engine, engine::general_purpose::STANDARD};
use std::fmt::Write;
use taffy::TaffyTree;

use crate::{
    error::Result,
    fonts::{LYRIC_FONT, LYRIC_FONT_SIZE, OCTAVE_FONT_SIZE, SOLFA_FONT, SOLFA_FONT_SIZE},
    layout::{A4_HEIGHT_PX, A4_WIDTH_PX},
    types::{Element, ElementKind, TextElement},
};

pub struct SvgEmitter {
    tree: TaffyTree<()>,
    svg: String,
}

impl SvgEmitter {
    pub fn new(tree: TaffyTree<()>) -> Self {
        Self {
            tree,
            svg: String::with_capacity(16 * 1024),
        }
    }

    pub fn render_to_svg(mut self, elements: &[Element]) -> Result<String> {
        writeln!(
            self.svg,
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {A4_WIDTH_PX} {A4_HEIGHT_PX}" width="{A4_WIDTH_PX}px" height="{A4_HEIGHT_PX}px">"#
        )?;

        self.emit_defs()?;

        for element in elements {
            match &element.kind {
                ElementKind::Text(text) => self.emit_text(text),
                _ => todo!(),
            }
        }

        self.svg.push_str("</svg>");

        Ok(self.svg)
    }

    fn emit_defs(&mut self) -> Result<()> {
        let solfa_font_b64 = STANDARD.encode(SOLFA_FONT);
        let lyrics_font_b64 = STANDARD.encode(LYRIC_FONT);

        writeln!(
            self.svg,
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
      font-family: 'FiraSans-Solfa', monospace;
      font-weight: bold;
      font-size: {SOLFA_FONT_SIZE}px;
      fill: #1a1a1a;
    }}

    .octave {{
      font-size: {OCTAVE_FONT_SIZE}px;
      font-weight: bold;
      fill: #a1a1a;
    }}
  </style>
</defs>"#
        )?;

        Ok(())
    }

    fn emit_text(&mut self, text: &TextElement) {
        let escaped_content = Self::xml_escape(&text.content);
        // let _ = writeln!(
        //     self.svg,
        //     r#"  <text x="{:.2}" y="{:.2}" class="{}">{}</text>"#,
        //     text.x, text.y, text.class, escaped_content
        // );
    }

    // fn emit_barline(svg: &mut String, barline: &BarlineElement) {
    //     let _ = writeln!(
    //         svg,
    //         r#"  <line x1="{:.2}" y1="{:.2}" x2="{:.2}" y2="{:.2}" stroke='#1a1a1a' stroke-width="1.5" />"#,
    //         barline.x, barline.y1, barline.x, barline.y2
    //     );
    // }
    //
    // fn emit_underline(svg: &mut String, underline: &UnderlineElement) {
    //     let _ = writeln!(
    //         svg,
    //         r#"  <line x1="{:.2}" y1="{:.2}" x2="{:.2}" y2="{:.2}" stroke='#1a1a1a' stroke-width="1.0" />"#,
    //         underline.x1, underline.y, underline.x2, underline.y
    //     );
    // }

    fn xml_escape(input: &str) -> String {
        let mut escaped = String::with_capacity(input.len());

        for c in input.chars() {
            match c {
                '<' => escaped.push_str("&lt;"),
                '>' => escaped.push_str("&gt;"),
                '&' => escaped.push_str("&amp;"),
                '"' => escaped.push_str("&quot;"),
                '\'' => escaped.push_str("&apos;"),
                _ => escaped.push(c),
            }
        }

        escaped
    }
}

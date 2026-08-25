use base64::{Engine, engine::general_purpose::STANDARD};
use std::fmt::Write;
use taffy::{NodeId, TaffyTree};

use crate::{
    error::Result,
    fonts::{LYRIC_FONT, LYRIC_FONT_SIZE, OCTAVE_FONT_SIZE, SOLFA_FONT, SOLFA_FONT_SIZE},
    layout::{A4_HEIGHT_PX, A4_WIDTH_PX, UNDERLINE_Y_OFFSET},
    types::{Element, ElementKind, TextElement, UnderlineElement},
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
                ElementKind::Text(text) => self.emit_text(element.node_id, text)?,
                ElementKind::Barline => self.emit_barline(element.node_id)?,
                ElementKind::Underline(elem) => self.emit_underline(element.node_id, elem)?,
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
      dominant-baseline: alphabetic;
    }}

    .solfa {{
      font-family: 'FiraSans-Solfa', monospace;
      font-weight: bold;
      font-size: {SOLFA_FONT_SIZE}px;
      fill: #1a1a1a;
      dominant-baseline: hanging;
    }}

    .octave {{
      font-size: {OCTAVE_FONT_SIZE}px;
      font-weight: bold;
      fill: #1a1a1a;
      dominant-baseline: alphabetic;
    }}
  </style>
</defs>"#
        )?;

        Ok(())
    }

    fn resolve_position(&self, node_id: NodeId) -> Result<(f32, f32)> {
        let mut x = 0.0;
        let mut y = 0.0;
        let mut current_node = Some(node_id);

        while let Some(node) = current_node {
            let layout = self.tree.layout(node)?;

            x += layout.location.x;
            y += layout.location.y;
            current_node = self.tree.parent(node);
        }

        Ok((x, y))
    }

    fn emit_text(&mut self, node_id: NodeId, text: &TextElement) -> Result<()> {
        let escaped_content = Self::xml_escape(&text.content);
        let (x, y) = self.resolve_position(node_id)?;

        writeln!(
            self.svg,
            r#"  <text x="{:.2}" y="{:.2}" class="{}">{}</text>"#,
            x, y, text.class, escaped_content
        )?;

        Ok(())
    }

    fn emit_barline(&mut self, node_id: NodeId) -> Result<()> {
        let (x, y1) = self.resolve_position(node_id)?;
        let layout = self.tree.layout(node_id)?;
        let y2 = y1 + layout.content_box_height();

        writeln!(
            self.svg,
            r#"  <line x1="{x:.2}" y1="{y1:.2}" x2="{x:.2}" y2="{y2:.2}" stroke='#000000' stroke-width="2.0" />"#,
        )?;

        Ok(())
    }

    fn emit_underline(&mut self, node_id: NodeId, underline: &UnderlineElement) -> Result<()> {
        let (x1, y) = self.resolve_position(node_id)?;
        let layout = self.tree.layout(node_id)?;
        let y = y + layout.content_box_height() + UNDERLINE_Y_OFFSET;
        let (x2, _) = self.resolve_position(underline.end_node)?;
        let x2 = x2 + underline.real_width;

        writeln!(
            self.svg,
            r#"  <line x1="{x1:.2}" y1="{y:.2}" x2="{x2:.2}" y2="{y:.2}" stroke='#000000' stroke-width="1.0" />"#,
        )?;

        Ok(())
    }

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

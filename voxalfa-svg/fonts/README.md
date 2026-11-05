# Fonts

The renderer embeds subsetted TrueType fonts so the generated SVG is portable.

| File                       | Role                                    | Source                                |
| -------------------------- | --------------------------------------- | ------------------------------------- |
| `FiraSans-Solfa.ttf`       | tonic solfa, accents, octaves           | Fira Sans Bold                        |
| `NotoSans-Lyrics.ttf`      | lyrics                                  | Noto Sans Condensed Regular           |
| `NotoSans-Regular.ttf`     | authors, meta information, page numbers | Noto Sans Regular                     |
| `NotoSans-Bold.ttf`        | title, key labels, jumps                | Noto Sans Bold                        |
| `NotoSerif-Italic.ttf`     | Tempo, Fine, Coda, To Coda marks        | Noto Serif Italic                     |
| `NotoSerif-BoldItalic.ttf` | dynamics                                | Noto Serif Bold Italic                |
| `NotoSansSymbols-Bold.ttf` | flat/sharp/natural glyph fallback       | Noto Sans Symbols Bold (scaled 1.25×) |

The Noto Sans and Noto Serif faces are distributed under the SIL Open Font
License; see `OFL.txt`.

## Subsetting

Download the hinted static faces from
`https://github.com/notofonts/noto-fonts/tree/main/hinted/ttf`, then subset
them with `fonttools`:

```sh
UNICODES='U+0020-024F,U+0300-036F,U+2000-206F,U+20AC,U+266D,U+266F'

subset() {
  pyftsubset "$1" \
    --output-file="voxalfa-svg/fonts/$2" \
    --unicodes="$UNICODES" \
    --layout-features='kern,liga,clig,calt' \
    --no-hinting \
    --name-IDs='*' \
    --name-languages='*' \
    --name-legacy \
    --notdef-outline \
    --recommended-glyphs
}

subset NotoSans-Regular.ttf NotoSans-Regular.ttf
subset NotoSans-Bold.ttf NotoSans-Bold.ttf
subset NotoSerif-Regular.ttf NotoSerif-Regular.ttf
subset NotoSerif-Italic.ttf NotoSerif-Italic.ttf
subset NotoSerif-BoldItalic.ttf NotoSerif-BoldItalic.ttf

pyftsubset NotoSansSymbols-Bold.ttf \
  --output-file="voxalfa-svg/fonts/NotoSansSymbols-Bold.ttf" \
  --unicodes="U+266D-266F" \
  --no-hinting \
  --name-IDs='*' \
  --name-languages='*' \
  --name-legacy \
  --notdef-outline \
  --recommended-glyphs
```

# Fonts

The renderer embeds subsetted TrueType fonts so the generated SVG is portable.

| File                       | Role                                    | Source                                |
| -------------------------- | --------------------------------------- | ------------------------------------- |
| `FiraSans-Bold.ttf`        | tonic solfa, accents, octaves           | Fira Sans Bold                        |
| `NotoSans-Condensed.ttf`   | lyrics                                  | Noto Sans Condensed Regular           |
| `NotoSans-Regular.ttf`     | authors, meta information, page numbers | Noto Sans Regular                     |
| `NotoSans-Bold.ttf`        | title, key labels, jumps                | Noto Sans Bold                        |
| `NotoSerif-Regular.ttf`    | time signatures, tempo labels           | Noto Serif Regular                    |
| `NotoSerif-Italic.ttf`     | Fine, Coda, To Coda marks               | Noto Serif Italic                     |
| `NotoSerif-BoldItalic.ttf` | dynamics                                | Noto Serif Bold Italic                |
| `NotoSansSymbols-Bold.ttf` | flat/sharp/natural glyph fallback       | Noto Sans Symbols Bold (scaled 1.25×) |

## Licenses

- **Fira Sans**: SIL Open Font License 1.1, copyright The Mozilla Foundation
  and Telefonica S.A. See `FiraSans-OFL.txt`. “Fira Sans” is a trademark of
  The Mozilla Corporation.
- **Noto Sans, Noto Serif, Noto Sans Symbols**: SIL Open Font License 1.1,
  copyright The Noto Project Authors. See `Noto-OFL.txt`.

## Subsetting

The embedded fonts are generated from the upstream font sources by
`subset.py`:

```sh
python3 -m pip install 'fonttools==4.63.0'
python3 voxalfa-svg/fonts/subset.py
```

The checked-in files were generated with fonttools 4.63.0.

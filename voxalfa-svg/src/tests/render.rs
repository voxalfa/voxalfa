use crate::{
    RenderOptions,
    fonts::{FontFace, FontFaces},
    render_pages, render_pages_with, render_pages_with_fonts,
};
use voxalfa_core::{output::FinalOutput, validation::Validator};

fn render(output: &FinalOutput) -> crate::Result<String> {
    render_pages(output).map(|pages| pages.join("\n"))
}

fn render_with(output: &FinalOutput, options: RenderOptions) -> crate::Result<String> {
    render_pages_with(output, options).map(|pages| pages.join("\n"))
}

fn analyze(source: &str) -> FinalOutput {
    let mut validator = Validator::init().unwrap();
    let output = validator.analyze(source);

    assert!(
        !output.has_error(),
        "unexpected diagnostics: {:#?}",
        output.diagnostics
    );

    output
}

#[test]
fn test_notes_and_lyrics() {
    let output = analyze(
        r#"

[#] title="Render test"
[#] verses={1}

[$] key={C} | time={4,4} | tempo={100} | voices={S,A}

---

[S] |d :r !m :f ||
[A] |m :f !s :l ||

[1] do re mi fa

"#,
    );

    let svg = render(&output).unwrap();

    assert!(svg.contains("<svg"));
    assert!(svg.contains("class=\"note-text\">d</text>"));
    assert!(svg.contains("class=\"lyrics-text\">do</text>"));
}

#[test]
fn test_wildcard_lyrics() {
    let output = analyze(
        r#"

[#] title="Wildcard lyrics"
[#] verses={2}

[$] key={C} | time={4,4} | tempo={100} | voices={S}

---

[S] |d :r !m :f ||

[*] do re mi fa
"#,
    );

    let svg = render(&output).unwrap();

    assert_eq!(svg.matches(r#"class="lyrics-text">do</text>"#).count(), 2);
}

#[test]
fn test_per_voice_wildcard_lyrics() {
    let output = analyze(
        r#"

[#] title="Per-voice wildcard"
[#] verses={2}

[$] key={C} | time={4,4} | tempo={100} | voices={S,A}

---

[S] |d :r !m :f ||
[*] sun moon star sky

++

[A] |d :r !m :f ||
[*] red green blue gold
"#,
    );

    let svg = render(&output).unwrap();

    assert_eq!(svg.matches(r#"class="lyrics-text">sun</text>"#).count(), 2);
    assert_eq!(svg.matches(r#"class="lyrics-text">red</text>"#).count(), 2);
}

#[test]
fn test_lyric_hyphen() {
    let output = analyze(
        r#"

[#] verses={1}

[$] key={C} | time={4,4} | tempo={100} | voices={S}

---

[S] |d :r !m ||

[1] har_ mo_ ny.
"#,
    );

    let svg = render(&output).unwrap();

    assert!(svg.contains(r#"class="lyrics-text">har-</text>"#));
    assert!(svg.contains(r#"class="lyrics-text">mo-</text>"#));
    assert!(svg.contains(r#"class="lyrics-text">ny.</text>"#));
}
#[test]
fn test_header_layout() {
    let output = analyze(
        r#"

[#] title="Header layout"
[#] author={"Alice"}
[#] composer={"Bob"}
[#] language="fr"
[#] meter={8,8,8,8}
[#] verses={1}

[$] key={C} | time={4,4} | tempo={100} | voices={S}

---

[S] |d :r !m :f ||

[1] do re mi fa
"#,
    );

    let svg = render(&output).unwrap();

    assert!(svg.contains(r#"text-anchor="middle" class="title-text">Header layout</text>"#));
    assert!(svg.contains(r#"text-anchor="start" class="author-text">Alice</text>"#));
    assert!(svg.contains(r#"text-anchor="end" class="composer-text">Bob</text>"#));
    assert!(svg.contains(r#"text-anchor="start" class="key-text">Do est C</text>"#));
    assert!(svg.contains(r#"text-anchor="start" class="time-text">4/4</text>"#));
    assert!(svg.contains(r#"text-anchor="start" class="bpm-label">M. 100.</text>"#));
    assert!(svg.contains(r#"text-anchor="middle" class="meter-text">8.8.8.8</text>"#));
}

#[test]
fn test_custom_header_rows() {
    let output = analyze(
        r#"
;; @header-row center="{{title}}" | center-style="title-text"
;; @header-row left={"Author: ", "{{author}}"} | left-style={"header-text", "author-text"} | right={"Music: ", "{{composer}}"} | right-style={"header-text", "composer-text"}
;; @header-row left={"{{key}}", "{{time}}", "{{tempo}}"} | left-style={"key-text", "time-text", "tempo-label"}

[#] title="Custom header"
[#] author={"Alice"}
[#] composer={"Bob"}
[#] language="en"
[#] meter={4,4}
[#] verses={1}

[$] key={C} | time={4,4} | tempo={allegro} | voices={S}

---

[S] |d :r !m :f ||

[1] do re mi fa
"#,
    );

    let svg = render(&output).unwrap();

    assert!(svg.contains(r#"class="title-text">Custom header</text>"#));
    assert!(svg.contains(r#"class="header-text">Author: </text>"#));
    assert!(svg.contains(r#"class="author-text">Alice</text>"#));
    assert!(svg.contains(r#"class="composer-text">Bob</text>"#));
    assert!(svg.contains(r#"class="key-text">Do is C</text>"#));
    assert!(svg.contains(r#"class="time-text">4/4</text>"#));
    assert!(svg.contains(r#"class="tempo-label">allegro.</text>"#));
    assert!(!svg.contains(r#"class="meter-text""#));
}

#[test]
fn test_style_merges_with_defaults() {
    let output = analyze(
        r#"
;; @style name="key-text" | color="red"

[#] title="Merged style"
[#] verses={1}

[$] key={C} | time={4,4} | tempo={100} | voices={S}

---

[S] |d :r !m :f ||

[1] do re mi fa
"#,
    );
    let mut options = RenderOptions::default();
    options.apply_directives(&output.header.directives);

    assert_eq!(options.elements.key.color.as_deref(), Some("red"));
    assert_eq!(
        options.elements.key.margin.right,
        crate::renderer::constants::HEADER_KEY_RIGHT_MARGIN
    );
}

#[test]
fn test_custom_style() {
    let output = analyze(
        r#"
;; @style name="credit" | font-size={10} | color="red"
;; @header-row left="Author: {{author}}" | left-style="credit"

[#] title="Custom style"
[#] author={"Alice"}
[#] verses={1}

[$] key={C} | time={4,4} | tempo={100} | voices={S}

---

[S] |d :r !m :f ||

[1] do re mi fa
"#,
    );

    let svg = render(&output).unwrap();

    assert!(svg.contains(".credit {"));
    assert!(svg.contains(r#"class="credit">Author: Alice</text>"#));
}

#[test]
fn test_custom_header_literal_braces() {
    let output = analyze(
        r#"
;; @header-row left={"{{author}}", " {literal}"} | left-style={"author-text", "header-text"}

[#] title="Braces"
[#] author={"Alice"}
[#] verses={1}

[$] key={C} | time={4,4} | tempo={100} | voices={S}

---

[S] |d :r !m :f ||

[1] do re mi fa
"#,
    );

    let svg = render(&output).unwrap();

    assert!(svg.contains(r#"class="author-text">Alice</text>"#));
    assert!(svg.contains("{literal}</text>"));
}

#[test]
fn test_header_row_style_spacing() {
    let output = analyze(
        r#"
;; @style name="row-gap" | margin-bottom={40}
;; @header-row left="First" | style="row-gap"
;; @header-row left="Second"

[#] title="Spacing"
[#] verses={1}

[$] key={C} | time={4,4} | tempo={100} | voices={S}

---

[S] |d :r !m :f ||

[1] do re mi fa
"#,
    );

    let svg = render(&output).unwrap();
    let y_of = |text: &str| {
        svg.lines()
            .find(|line| line.contains(text))
            .and_then(|line| line.split("y=\"").nth(1))
            .and_then(|value| value.split('"').next())
            .and_then(|value| value.parse::<f32>().ok())
            .expect("header text y position")
    };

    assert!(y_of("First") + 40.0 <= y_of("Second"));
}

#[test]
fn test_custom_header_wraps() {
    let output = analyze(
        r#"
;; @page width={220} | height={800} | margin={20}
;; @header-row left="A very long left header that must wrap" | right="Right side"

[#] title="Wrapping"
[#] verses={1}

[$] key={C} | time={4,4} | tempo={100} | voices={S}

---

[S] |d :r !m :f ||

[1] do re mi fa
"#,
    );

    let svg = render(&output).unwrap();

    let header = svg
        .split("<text x=")
        .filter(|part| part.contains(r#"class="header-text""#))
        .count();

    assert!(header >= 3, "expected the long header cell to wrap");
}

#[test]
fn test_header_tempo() {
    let output = analyze(
        r#"

[#] title="Header tempo"
[#] verses={1}

[$] key={C} | time={4,4} | tempo={allegro} | voices={S}

---

[S] |d :r !m :f ||

[1] do re mi fa
"#,
    );

    let svg = render(&output).unwrap();

    assert!(svg.contains(r#"text-anchor="start" class="tempo-label">allegro.</text>"#));
}

#[test]
fn test_time_and_jumps() {
    let output = analyze(
        r#"

[#] title="Markers"

[$] key={C} | time={4,4} | tempo={100} | voices={S}

---

[S] |d :r !m :f ||

--

[$] time={3,4} | jump={DS} | mark={S}

[S] |d :r :m ||

--

[$] jump={DSC} | mark={F} | repeat={2}

[S] |d :r :m ||

--

[$] mark={C}

[S] |d :r :m ||
"#,
    );

    let svg = render(&output).unwrap();

    assert!(svg.contains(r#"class="time-label">3/4</text>"#));
    assert!(svg.contains(r#"class="jump-label">D.S.</text>"#));
    assert!(svg.contains(r#"class="jump-label">D.S. al Coda x 2</text>"#));
    assert!(svg.contains(r#"class="mark-label">Fine</text>"#));
    assert!(svg.contains(r#"class="mark-label">Coda</text>"#));
    assert!(svg.contains(r#"class="marker marker-segno""#));
}

#[test]
fn test_accidentals() {
    let output = analyze(
        r#"

[#] title="Accidental keys"
[#] language="en"

[$] key={Bb} | time={4,4} | tempo={100} | voices={S}

---

[S] |d :r !m :f ||

--

[$] key={Ab}

[S] |d :r !m :f ||

--

[$] key={F#}

[S] |d :r !m :f ||
"#,
    );

    let svg = render(&output).unwrap();

    assert!(svg.contains(r#"class="key-text">Do is B♭</text>"#));
    assert!(svg.contains(r#"class="key-label">Do is A♭</text>"#));
    assert!(svg.contains(r#"class="key-label">Do is F♯</text>"#));
}

#[test]
fn test_touches() {
    let output = analyze(
        r#"

[#] title="Touches"

[$] key={C} | time={4,4} | tempo={100} | voices={S}

---

[S] |d :r !m :f ||

--

[$] touches={fermata:0, staccato:1, marcato:2}

[S] |d :r !m :f ||
"#,
    );

    let svg = render(&output).unwrap();

    assert!(svg.contains(r#"class="touch-fermata""#));
    assert!(svg.contains(r#"class="touch-fill touch-staccato""#));
    assert!(svg.contains(r#"class="touch-fill touch-marcato""#));
}

#[test]
fn test_hairpin_merged_sections() {
    let output = analyze(
        r#"

[#] title="Merged hairpin"

[$] key={C} | time={3,4} | tempo={100} | voices={S}

---

[$] dynamics={cresc:0..5}

[S] |d :- .d :d ||

<<

[S] |m :- .m :m ||
"#,
    );

    let svg = render(&output).unwrap();

    assert!(svg.contains(r#"class="dynamic-hairpin""#));
    assert!(!svg.contains(r#"class="dynamic-dash""#));
}

#[test]
fn test_hairpin_continuation() {
    let output = analyze(
        r#"

;; @page width={200} | height={1000}

[#] title="Split hairpin"

[$] key={C} | time={3,4} | tempo={100} | voices={S}

---

[$] dynamics={cresc:0..5}

[S] |d :- .d :d ||

<<

[S] |m :- .m :m ||
"#,
    );

    let svg = render(&output).unwrap();

    assert_eq!(
        svg.matches(r#"class="dynamic-label">cresc.</text>"#)
            .count(),
        1
    );
    assert_eq!(svg.matches(r#"class="dynamic-dash""#).count(), 2);
}

#[test]
fn test_key_tempo_dynamics() {
    let output = analyze(
        r#"

[#] title="Labels"
[#] language="en"

[$] key={C} | time={4,4} | tempo={100} | voices={S}

---

[S] |d :r !m :f ||

--

[$] key={D} | tempo={allegro}
[$] dynamics={p:0, mf:2, cresc:1..2}

[S] |d :r !m :f ||

--

[$] tempo={120}

[S] |d :r !m :f ||
"#,
    );

    let svg = render(&output).unwrap();

    assert!(svg.contains(r#"class="key-label">Do is D</text>"#));
    assert!(svg.contains(r#"class="tempo-label">allegro.</text>"#));
    assert!(svg.contains(r#"class="bpm-label">M. 120.</text>"#));
    assert!(svg.contains(
        ".bpm-label {\n      font-family: 'Noto Serif', serif;\n      font-size: 11.00px;\n      font-weight: 400;\n      font-style: normal;"
    ));
    assert!(svg.contains(r#"class="dynamic-label">p</text>"#));
    assert!(svg.contains(r#"class="dynamic-label">mf</text>"#));
    assert!(!svg.contains(r#"class="dynamic-label">cresc</text>"#));
}

#[test]
fn test_volta_endings() {
    let output = analyze(
        r#"

[#] title="Volta"

[$] key={C} | time={4,4} | tempo={100} | voices={S}

---

[S] |d :r !m :f ||

--

[$] ending={1} | label="First" | jump={DS}

[S] |d :r !m :f ||

--

[$] ending={2}

[S] |d :r !m :f ||
"#,
    );

    let svg = render(&output).unwrap();

    assert!(svg.contains(r#"class="volta""#));
    assert!(svg.contains(r#"class="volta-label">1. First</text>"#));
    assert!(svg.contains(r#"class="volta-label">2.</text>"#));
}

#[test]
fn test_multi_voice_lyrics() {
    let output = analyze(
        r#"

[#] title="Multi lyric"
[#] verses={1}

[$] key={C} | time={4,4} | tempo={100} | voices={S,A,T,B}

---

[S] |d :r !m :f ||
[A] |m :f !s :l ||

[1] do re mi fa

++

[T] |d :r !m :f ||
[B] |m :f !s :l ||

[1] la ti do re

"#,
    );

    let svg = render(&output).unwrap();

    assert!(svg.contains(">do<"));
    assert!(svg.contains(">la<"));
}

#[test]
fn test_virtual_columns() {
    let output = analyze(
        r#"

[#] title="Expanded pulses"
[#] verses={1}

[$] key={C} | time={4,4} | tempo={100} | voices={S,A}

---

[S] |d.r :m ||
[A] | :m ||

[1] do re
"#,
    );

    let svg = render(&output).unwrap();

    // The second voice's first pulse is expanded from the first voice's
    // subdivision. Its virtual columns must not render dividers.
    assert_eq!(svg.matches(r#"class="divider-text">.</text>"#).count(), 1);
}

#[test]
fn test_element_fonts() {
    let output = analyze(
        r#"
;; @style name="note-text" | font-family={"Custom Note"} | font-size={18}
;; @style name="lyrics-text" | font-size={9}
;; @style name="jump-label" | font-size={15}

[#] title="Directives"
[#] verses={1}

[$] key={C} | time={4,4} | tempo={100} | voices={S}

---

[S] |d :r !m :f ||

[1] do re mi fa
"#,
    );

    let mut options = RenderOptions::default();
    options.apply_directives(&output.header.directives);

    assert_eq!(
        options.elements.note.font_family.as_deref(),
        Some("Custom Note")
    );
    assert_eq!(options.elements.note.font_size, Some(18.0));
    assert_eq!(options.elements.lyrics.font_size, Some(9.0));
    assert_eq!(options.elements.jump.font_size, Some(15.0));

    let svg = render_with(&output, options).unwrap();

    assert!(svg.contains("font-family: 'Custom Note'"));
    assert!(svg.contains("font-family: 'Custom Note', monospace;"));
    assert!(svg.contains("font-size: 18.00px;"));
    assert!(svg.contains(".lyrics-text {"));
    assert!(svg.contains("font-size: 9.00px;"));
}

#[test]
fn test_element_colors() {
    let output = analyze(
        r##"
;; @style name="title-text" | color={"#ff0000"}
;; @style name="time-text" | color={"rebeccapurple"}

[#] title="Colored"
[#] verses={1}

[$] key={C} | time={4,4} | tempo={100} | voices={S}

---

[S] |d :r !m :f ||

[1] do re mi fa
"##,
    );

    let mut options = RenderOptions::default();
    options.apply_directives(&output.header.directives);

    assert_eq!(options.elements.title.color.as_deref(), Some("#ff0000"));
    assert_eq!(
        options.elements.time.color.as_deref(),
        Some("rebeccapurple")
    );

    let svg = render_with(&output, options).unwrap();

    assert!(svg.contains(".title-text {"));
    assert!(svg.contains("      fill: #ff0000;"));
    assert!(svg.contains("      fill: rebeccapurple;"));
    assert!(svg.contains(".footer-text {"));
    assert!(svg.contains("      fill: black;"));
    assert!(svg.contains(".note-text {"));
    assert!(svg.contains(
        ".note-text {\n      font-family: 'Fira Sans', monospace;\n      font-size: 12.00px;\n      font-weight: 700;\n      font-style: normal;\n      fill: black;"
    ));
}

#[test]
fn test_font_weight_style() {
    let output = analyze(
        r#"
;; @style name="title-text" | font-weight={"normal"}
;; @style name="tempo-label" | font-style={"normal"}
;; @style name="bpm-label" | font-style={"italic"}

[#] title="Font styles"
[#] verses={1}

[$] key={C} | time={4,4} | tempo={100} | voices={S}

---

[S] |d :r !m :f ||

[1] do re mi fa
"#,
    );

    let svg = render(&output).unwrap();

    assert!(svg.contains(
        ".title-text {\n      font-family: 'Noto Sans', sans-serif;\n      font-size: 18.00px;\n      font-weight: 400;\n      font-style: normal;"
    ));
    assert!(svg.contains(
        ".tempo-label {\n      font-family: 'Noto Serif', serif;\n      font-size: 11.00px;\n      font-weight: 400;\n      font-style: normal;"
    ));
    assert!(svg.contains(
        ".bpm-label {\n      font-family: 'Noto Serif', serif;\n      font-size: 11.00px;\n      font-weight: 400;\n      font-style: italic;"
    ));
    assert!(svg.contains(
        ".mark-label {\n      font-family: 'Noto Serif', serif;\n      font-size: 11.00px;\n      font-weight: 400;\n      font-style: italic;"
    ));
    assert!(svg.contains(
        ".note-text {\n      font-family: 'Fira Sans', monospace;\n      font-size: 12.00px;\n      font-weight: 700;\n      font-style: normal;"
    ));
}

#[test]
fn test_face_set_fonts() {
    const CUSTOM_FONT: &[u8] = include_bytes!("../../fonts/NotoSans-Regular.ttf");

    let output = analyze(
        r#"
;; @style name="note-text" | font-family={"Custom Note"} | font-weight={"normal"} | font-style={"italic"}

[#] title="Custom faces"
[#] verses={1}

[$] key={C} | time={4,4} | tempo={100} | voices={S}

---

[S] |d :r !m :f ||

[1] do re mi fa
"#,
    );

    let faces = FontFaces::default().with(FontFace::new("Custom Note", CUSTOM_FONT));
    let svg = render_pages_with_fonts(&output, &RenderOptions::default(), &faces)
        .unwrap()
        .join("\n");

    assert!(svg.contains("font-family: 'Custom Note', monospace;"));
    assert!(svg.contains(
        ".note-text {\n      font-family: 'Custom Note', monospace;\n      font-size: 12.00px;\n      font-weight: 400;\n      font-style: italic;"
    ));
    assert!(svg.contains(
        "@font-face {\n      font-family: 'Custom Note';\n      src: url('data:font/ttf;base64,"
    ));
}

#[test]
fn test_header_box() {
    let output = analyze(
        r#"
;; @page width={800} | height={1000} | margin-x={30} | margin-y={40}
;; @style name="title-text" | margin-top={10} | padding-x={4}
;; @style name="author-text" | margin-bottom={3}

[#] title="Boxed header"
[#] author={"Alice"}

[$] key={C} | time={4,4} | tempo={100} | voices={S}

---

[S] |d :r !m :f ||
"#,
    );

    let mut options = RenderOptions::default();

    options.apply_directives(&output.header.directives);

    assert_eq!(options.page.width, 800.0);
    assert_eq!(options.page.height, 1000.0);
    assert_eq!(options.page_margin.left, 30.0);
    assert_eq!(options.page_margin.right, 30.0);
    assert_eq!(options.page_margin.top, 40.0);
    assert_eq!(options.page_margin.bottom, 40.0);
    assert_eq!(options.elements.title.margin.top, 10.0);
    assert_eq!(options.elements.title.padding.left, 4.0);
    assert_eq!(options.elements.title.padding.right, 4.0);
    assert_eq!(options.elements.author.margin.bottom, 3.0);

    let svg = render_with(&output, options).unwrap();

    assert!(svg.contains(r#"viewBox="0 0 800.00 1000.00""#));
    assert!(svg.contains("class=\"title-text\""));
    assert!(svg.contains("class=\"author-text\""));
}

#[test]
fn test_font_stretch() {
    let output = analyze(
        r#"
;; @style name="note-text" | font-stretch={"condensed"}

[#] title="Stretch"
[#] verses={1}

[$] key={C} | time={4,4} | tempo={100} | voices={S}

---

[S] |d :r !m :f ||

[1] do re mi fa
"#,
    );

    let mut options = RenderOptions::default();
    options.apply_directives(&output.header.directives);

    assert_eq!(
        options.elements.note.font_stretch,
        Some(voxalfa_core::ast::directives::FontStretch::Condensed)
    );

    let svg = render_with(&output, options).unwrap();

    assert!(svg.contains("font-stretch: condensed;"));
}

#[test]
fn test_printable_width() {
    let output = analyze(
        r#"
;; @page width={800} | height={1000} | margin-x={30} | margin-y={40}

[#] title="Brace bounds"
[#] verses={1}

[$] key={C} | time={4,4} | tempo={100} | voices={S,A}

---

[S] |d :r !m :f ||
[A] |m :f !s :l ||

[1] do re mi fa
"#,
    );

    let svg = render(&output).unwrap();

    for path in svg.split("<path d=\"").skip(1).filter_map(|chunk| {
        chunk
            .split_once("\" class=\"brace\"/>")
            .map(|(path, _)| path)
    }) {
        let numbers = path
            .split_whitespace()
            .filter_map(|token| token.parse::<f32>().ok())
            .collect::<Vec<_>>();

        for x in numbers.iter().step_by(2) {
            assert!(
                (30.0..=770.0).contains(x),
                "brace coordinate {x} escaped the printable width in {path}"
            );
        }
    }
}

#[test]
fn test_footer_page_number() {
    let output = analyze(
        r#"

;; @page width={200} | height={200} | margin-x={30} | margin-y={30}
;; @style name="footer-text" | font-size={30}

[#] title="Footer"

[$] key={C} | time={4,4} | tempo={100} | voices={S}

---

[S] |d :r !m :f ||

--

[S] |d :r !m :f ||

--

[S] |d :r !m :f ||

--

[S] |d :r !m :f ||

--

[S] |d :r !m :f ||

--

[S] |d :r !m :f ||
"#,
    );

    let svg = render(&output).unwrap();

    assert!(svg.contains(".footer-text {"));
    assert!(svg.contains("font-size: 30.00px"));
    assert!(svg.contains(r#"class="footer-text">1</text>"#));
    assert!(svg.contains(r#"class="footer-text">2</text>"#));
    assert!(!svg.contains(r#"class="footer-text">1 /"#));
}

#[test]
fn test_single_voice_curly() {
    let output = analyze(
        r#"

[#] title="Single voice braces"

[$] key={C} | time={4,4} | tempo={100} | voices={S}

---

[S] |d :r !m :f ||
"#,
    );

    let svg = render(&output).unwrap();

    assert_eq!(svg.matches(r#"class="brace"/>"#).count(), 1);
}

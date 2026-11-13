use voxalfa_core::{output::FinalOutput, validation::Validator};
use voxalfa_svg::{
    RenderOptions,
    fonts::{FontFace, FontFaces},
    render_pages, render_pages_with, render_pages_with_fonts,
};

fn render(output: &FinalOutput) -> voxalfa_svg::Result<String> {
    render_pages(output).map(|pages| pages.join("\n"))
}

fn render_with(output: &FinalOutput, options: RenderOptions) -> voxalfa_svg::Result<String> {
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
fn renders_notes_and_lyrics() {
    let output = analyze(
        r#"
;; @version 0.1.0-alpha

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
fn repeats_wildcard_lyrics_for_every_verse() {
    let output = analyze(
        r#"
;; @version 0.1.0-alpha

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
fn repeats_per_voice_wildcard_lyrics_for_every_verse() {
    let output = analyze(
        r#"
;; @version 0.1.0-alpha

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
fn appends_hyphen_to_concatenated_lyrics() {
    let output = analyze(
        r#"
;; @version 0.1.0-alpha

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
fn renders_header_in_requested_layout() {
    let output = analyze(
        r#"
;; @version 0.1.0-alpha

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
fn renders_header_tempo_label() {
    let output = analyze(
        r#"
;; @version 0.1.0-alpha

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
fn renders_time_signature_and_jump_markers() {
    let output = analyze(
        r#"
;; @version 0.1.0-alpha

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
fn renders_accidentals() {
    let output = analyze(
        r#"
;; @version 0.1.0-alpha

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
fn renders_fermata_staccato_and_marcato_touches() {
    let output = analyze(
        r#"
;; @version 0.1.0-alpha

[#] title="Touches"

[$] key={C} | time={4,4} | tempo={100} | voices={S}

---

[S] |d :r !m :f ||

--

[$] touches={frm:0, stc:1, mrc:2}

[S] |d :r !m :f ||
"#,
    );

    let svg = render(&output).unwrap();

    assert!(svg.contains(r#"class="touch-fermata""#));
    assert!(svg.contains(r#"class="touch-fill touch-staccato""#));
    assert!(svg.contains(r#"class="touch-fill touch-marcato""#));
}

#[test]
fn renders_dynamic_hairpin_across_merged_sections() {
    let output = analyze(
        r#"
;; @version 0.1.0-alpha

[#] title="Merged hairpin"

[$] key={C} | time={3,4} | tempo={100} | voices={S}

---

[$] dynamics={cre:0..6}

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
fn continues_hairpin_dash_without_repeating_label_across_systems() {
    let output = analyze(
        r#"
;; @version 0.1.0-alpha

;; @page width={200} | height={1000}

[#] title="Split hairpin"

[$] key={C} | time={3,4} | tempo={100} | voices={S}

---

[$] dynamics={cre:0..6}

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
fn renders_key_tempo_and_dynamics_labels() {
    let output = analyze(
        r#"
;; @version 0.1.0-alpha

[#] title="Labels"
[#] language="en"

[$] key={C} | time={4,4} | tempo={100} | voices={S}

---

[S] |d :r !m :f ||

--

[$] key={D} | tempo={allegro}
[$] dynamics={p:0, mf:2, cre:1..2}

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
        ".bpm-label {\n      font-family: 'Noto Serif', serif;\n      font-size: 12.00px;\n      font-weight: 400;\n      font-style: normal;"
    ));
    assert!(svg.contains(r#"class="dynamic-label">p</text>"#));
    assert!(svg.contains(r#"class="dynamic-label">mf</text>"#));
    assert!(!svg.contains(r#"class="dynamic-label">cre</text>"#));
}

#[test]
fn renders_volta_endings_with_labels() {
    let output = analyze(
        r#"
;; @version 0.1.0-alpha

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
fn resolves_multi_voice_lyrics_per_subsection() {
    let output = analyze(
        r#"
;; @version 0.1.0-alpha

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
fn skips_dividers_for_expanded_virtual_columns() {
    let output = analyze(
        r#"
;; @version 0.1.0-alpha

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
fn applies_per_element_font_directives() {
    let output = analyze(
        r#"
;; @version 0.1.0-alpha
;; @note-text font-family={"Custom Note"} | font-size={18}
;; @lyrics-text font-size={9}
;; @jump-label font-size={15}

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
fn applies_element_color_directives() {
    let output = analyze(
        r##"
;; @version 0.1.0-alpha
;; @title-text color={"#ff0000"}
;; @time-text color={"rebeccapurple"}

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
fn applies_font_weight_and_style_directives() {
    let output = analyze(
        r#"
;; @version 0.1.0-alpha
;; @title-text font-weight={"normal"}
;; @tempo-label font-style={"normal"}
;; @bpm-label font-style={"italic"}

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
        ".tempo-label {\n      font-family: 'Noto Serif', serif;\n      font-size: 12.00px;\n      font-weight: 400;\n      font-style: normal;"
    ));
    assert!(svg.contains(
        ".bpm-label {\n      font-family: 'Noto Serif', serif;\n      font-size: 12.00px;\n      font-weight: 400;\n      font-style: italic;"
    ));
    assert!(svg.contains(
        ".mark-label {\n      font-family: 'Noto Serif', serif;\n      font-size: 12.00px;\n      font-weight: 400;\n      font-style: italic;"
    ));
    assert!(svg.contains(
        ".note-text {\n      font-family: 'Fira Sans', monospace;\n      font-size: 12.00px;\n      font-weight: 700;\n      font-style: normal;"
    ));
}

#[test]
fn resolves_fonts_from_the_supplied_face_set() {
    const CUSTOM_FONT: &[u8] = include_bytes!("../fonts/NotoSans-Regular.ttf");

    let output = analyze(
        r#"
;; @version 0.1.0-alpha
;; @note-text font-family={"Custom Note"} | font-weight={"normal"} | font-style={"italic"}

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
fn applies_header_box_directives() {
    let output = analyze(
        r#"
;; @version 0.1.0-alpha
;; @page width={800} | height={1000} | margin-x={30} | margin-y={40}
;; @title-text margin-top={10} | padding-x={4}
;; @author-text margin-bottom={3}

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
fn applies_font_stretch_directive() {
    let output = analyze(
        r#"
;; @version 0.1.0-alpha
;; @note-text font-stretch={"condensed"}

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
fn keeps_braces_inside_the_printable_width() {
    let output = analyze(
        r#"
;; @version 0.1.0-alpha
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
fn renders_footer_as_page_number_only() {
    let output = analyze(
        r#"
;; @version 0.1.0-alpha

;; @page width={200} | height={200} | margin-x={30} | margin-y={30}
;; @footer-text font-size={30}

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
fn renders_curly_for_a_single_voice() {
    let output = analyze(
        r#"
;; @version 0.1.0-alpha

[#] title="Single voice braces"

[$] key={C} | time={4,4} | tempo={100} | voices={S}

---

[S] |d :r !m :f ||
"#,
    );

    let svg = render(&output).unwrap();

    assert_eq!(svg.matches(r#"class="brace"/>"#).count(), 1);
}

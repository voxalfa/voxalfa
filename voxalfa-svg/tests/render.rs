use voxalfa_core::validation::Validator;
use voxalfa_svg::{RenderOptions, render, render_with};

fn analyze(source: &str) -> voxalfa_core::output::FinalOutput {
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
    assert!(svg.contains(r#"text-anchor="middle" class="meter-text">8.8.8.8</text>"#));
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
fn renders_flat_keys_with_a_music_symbol() {
    let output = analyze(
        r#"
;; @version 0.1.0-alpha

[#] title="Flat keys"
[#] language="en"

[$] key={Bb} | time={4,4} | tempo={100} | voices={S}

---

[S] |d :r !m :f ||

--

[$] key={Ab}

[S] |d :r !m :f ||
"#,
    );

    let svg = render(&output).unwrap();

    assert!(svg.contains(r#"class="key-text">Do is B♭</text>"#));
    assert!(svg.contains(r#"class="key-label">Do is A♭</text>"#));
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
    assert!(svg.contains(r#"class="tempo-label">allegro</text>"#));
    assert!(svg.contains(r#"class="tempo-label">120</text>"#));
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
    assert!(svg.contains(r#"class="volta-text">1. First</text>"#));
    assert!(svg.contains(r#"class="volta-text">2.</text>"#));
}

#[test]
fn copies_shared_lyrics_when_voices_split() {
    let output = analyze(
        r#"
;; @version 0.1.0-alpha

[#] title="Copied lyrics"
[#] verses={1}

[$] key={C} | time={4,4} | tempo={100} | voices={S,A,T,B}

---

[S] |d :r !m :f ||
[A] |m :f !s :l ||
[T] |d :r !m :f ||
[B] |m :f !s :l ||

[1] Ma_ da_ gasi_ ka_ @@

--

[S] |d :- !- :s-1 ||
[A] |m-1 :- !- :s-1 ||

[1] soa@3\ Ny @@

++

[T] |d :- !- : ||
[1] soa@3\ ~ @@

++

[B] |d :- !- : ||
[1] soa@3\ ~ @@

--

[S] |d ||
[A] |m ||
[T] |d ||
[B] |m ||

[1] end
"#,
    );

    let svg = render(&output).unwrap();

    // The shared line is copied onto both split lyric rows, while the
    // S/A-only syllable stays in the first row.
    assert_eq!(svg.matches("class=\"lyrics-text\">Ma</text>").count(), 2);
    assert_eq!(svg.matches("class=\"lyrics-text\">soa</text>").count(), 2);
    assert_eq!(svg.matches("class=\"lyrics-text\">Ny</text>").count(), 1);
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
        options.elements.text.note.font_family.as_deref(),
        Some("Custom Note")
    );
    assert_eq!(options.elements.text.note.font_size, Some(18.0));
    assert_eq!(options.elements.text.lyrics.font_size, Some(9.0));
    assert_eq!(options.elements.label.jump.font_size, Some(15.0));

    let svg = render_with(&output, options).unwrap();

    assert!(svg.contains("font-family: 'Custom Note'"));
    assert!(svg.contains("font-family: 'Custom Note', monospace;"));
    assert!(svg.contains("font-size: 18.00px;"));
    assert!(svg.contains(".lyrics-text {"));
    assert!(svg.contains("font-size: 9.00px;"));
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
    assert_eq!(options.elements.text.title.margin.top, 10.0);
    assert_eq!(options.elements.text.title.padding.left, 4.0);
    assert_eq!(options.elements.text.title.padding.right, 4.0);
    assert_eq!(options.elements.text.author.margin.bottom, 3.0);

    let svg = render_with(&output, options).unwrap();

    assert!(svg.contains(r#"viewBox="0 0 800.00 1000.00""#));
    assert!(svg.contains("class=\"title-text\""));
    assert!(svg.contains("class=\"author-text\""));
}

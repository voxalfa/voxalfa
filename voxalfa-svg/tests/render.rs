use voxalfa_core::validation::Validator;
use voxalfa_svg::render;

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
    assert!(svg.contains("class=\"solfa\">d</text>"));
    assert!(svg.contains("class=\"lyric\">do</text>"));
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

    assert!(svg.contains(r#"text-anchor="middle" class="title">Header layout</text>"#));
    assert!(svg.contains(r#"text-anchor="start" class="name">Alice</text>"#));
    assert!(svg.contains(r#"text-anchor="end" class="name">Bob</text>"#));
    assert!(svg.contains(r#"text-anchor="start" class="key">Do est C</text>"#));
    assert!(svg.contains(r#"text-anchor="start" class="meta">4/4</text>"#));
    assert!(svg.contains(r#"text-anchor="middle" class="meta">8.8.8.8</text>"#));
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

    assert!(svg.contains(r#"class="marker-sans">3/4</text>"#));
    assert!(svg.contains(r#"class="marker-bold">D.S.</text>"#));
    assert!(svg.contains(r#"class="marker-bold">D.S. al Coda x 2</text>"#));
    assert!(svg.contains(r#"class="marker-italic">Fine</text>"#));
    assert!(svg.contains(r#"class="marker-italic">Coda</text>"#));
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

    assert!(svg.contains(r#"class="key">Do is B♭</text>"#));
    assert!(svg.contains(r#"class="marker-bold">Do is A♭</text>"#));
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

    assert!(svg.contains(r#"class="marker-bold">Do is D</text>"#));
    assert!(svg.contains(r#"class="marker-italic">allegro</text>"#));
    assert!(svg.contains(r#"class="marker-bold">120</text>"#));
    assert!(svg.contains(r#"class="dynamic">p</text>"#));
    assert!(svg.contains(r#"class="dynamic">mf</text>"#));
    assert!(!svg.contains(r#"class="dynamic">cre</text>"#));
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
    assert_eq!(svg.matches("class=\"lyric\">Ma</text>").count(), 2);
    assert_eq!(svg.matches("class=\"lyric\">soa</text>").count(), 2);
    assert_eq!(svg.matches("class=\"lyric\">Ny</text>").count(), 1);
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

use voxalfa_core::validation::Validator;
use voxalfa_svg2::render;

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
    assert!(svg.contains(r#"text-anchor="start" class="meta">Do est C</text>"#));
    assert!(svg.contains(r#"text-anchor="start" class="meta">4/4</text>"#));
    assert!(svg.contains(r#"text-anchor="middle" class="meta">8.8.8.8</text>"#));
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

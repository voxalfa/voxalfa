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

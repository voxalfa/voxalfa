use crate::tests::utils::assert_diagnostic_snapshot;

mod utils;

#[test]
fn test_syntax_error() {
    let source = r#"
[#] title="foo"
[#] author "bar"

---
"#;

    assert_diagnostic_snapshot("syntax_error", source);
}

#[test]
fn test_missing_error() {
    let source = r#"
[#] title="foo 

---
"#;

    assert_diagnostic_snapshot("missing_error", source);
}

#[test]
fn test_key_reassignment_error() {
    let source = r#"
[#] title="foo"
[#] title="bar"

---
"#;

    assert_diagnostic_snapshot("key_reassignment_error", source);
}

#[test]
fn test_unknown_metadata_error() {
    let source = r#"
[#] invalid="foo"

---
"#;

    assert_diagnostic_snapshot("unknown_metadata_error", source);
}

#[test]
fn test_unknown_parameter_error() {
    let source = r#"
[$] invalid="foo"

---
"#;

    assert_diagnostic_snapshot("unknown_parameter_error", source);
}

#[test]
fn test_type_exception_error() {
    let source = r#"
[#] title={1}

---
"#;

    assert_diagnostic_snapshot("type_exception_error", source);
}

#[test]
fn test_invalid_type_error() {
    let source = r#"
[$] key={Z}

---
"#;

    assert_diagnostic_snapshot("invalid_type_error", source);
}

#[test]
fn test_invalid_time_singature_error() {
    let source = r#"
[$] time={3,3,3}

---
"#;

    assert_diagnostic_snapshot("invalid_time_singature_error", source);
}

#[test]
fn test_timestamp_range_exception_error() {
    let source = r#"
[$] time={4,4} | voices={A}
---

[$] dynamics={cre:1}

[A] |d :r !m :f ||

"#;

    assert_diagnostic_snapshot("timestamp_range_exception_error", source);
}

#[test]
fn test_range_not_allowed_error() {
    let source = r#"
[$] time={4,4} | voices={A}
---

[$] dynamics={f:0..3}

[A] |d :r !m :f ||

"#;

    assert_diagnostic_snapshot("range_not_allowed_error", source);
}

#[test]
fn test_invalid_voice_error() {
    let source = r#"
[$] time={4,4} | voices={A}
---

[E] |d :r !m :f ||

"#;

    assert_diagnostic_snapshot("invalid_voice_error", source);
}

#[test]
fn test_invalid_note_distribution_error() {
    let source = r#"
[$] time={4,4} | voices={S}
---

[S] |d.d.r :r !m :f ||

"#;

    assert_diagnostic_snapshot("invalid_note_distribution_error", source);
}

#[test]
fn test_pulse_mismatch_error() {
    let source = r#"
[$] time={4,4} | voices={S,T}
---

[S] |d :r !m :f |s :l !t :d+1 ||
[T] |d :r !m :f ||

"#;

    assert_diagnostic_snapshot("pulse_mismatch_error", source);
}

#[test]
fn test_voice_mismatch_error() {
    let source = r#"
[$] time={4,4} | voices={S,T}
---

[S] |d :r !m :f |s :l !t :d+1 ||

"#;

    assert_diagnostic_snapshot("voice_mismatch_error", source);
}

#[test]
fn test_unmatched_underline_error() {
    let source = r#"
[$] time={4,4} | voices={S}
---

[S] |`d :r !m :f |s :l !t :d+1 ||

"#;

    assert_diagnostic_snapshot("unmatched_underline_error", source);
}

#[test]
fn test_mismatched_verse_warning() {
    let source = r#"
[#] verses={1}

[$] time={4,4} | voices={S}
---

[S] |d :r !m :f ||
[2] do re mi fa

"#;

    assert_diagnostic_snapshot("mismatched_verse_warning", source);
}

#[test]
fn test_missing_lyrics_anchor_error() {
    let source = r#"
[#] verses={1}

[$] time={4,4} | voices={S}
---

[S] |d :r !m :f ||
[1] do re mi fa\

--

[S] |d :r !m :f ||
[1] do re mi fa

"#;

    assert_diagnostic_snapshot("missing_lyrics_anchor_error", source);
}

#[test]
fn test_invalid_prolongation_error() {
    let source = r#"
[$] time={4,4} | voices={S}
---

[S] |- :r !m :f ||
"#;

    assert_diagnostic_snapshot("invalid_prolongation_error", source);
}

#[test]
fn test_pulse_accent_mismatch_error() {
    let source = r#"
[$] time={4,4} | voices={S}
---

[S] |d :r !m |f ||
"#;

    assert_diagnostic_snapshot("pulse_accent_mismatch_error", source);
}

#[test]
fn test_trailing_lyric_error() {
    let source = r#"
[#] verses={1}

[$] time={4,4} | voices={S}
---

[S] |d :r !m :f ||
[1] do re mi fa so
"#;

    assert_diagnostic_snapshot("trailing_lyric_error", source);
}

#[test]
fn test_verse_count_error() {
    let source = r#"
[#] verses={2}

[$] time={4,4} | voices={S}
---

[S] |d :r !m :f ||
[1] do re mi fa
"#;

    assert_diagnostic_snapshot("verse_count_error", source);
}

#[test]
fn test_undefined_verse_metdata_error() {
    let source = r#"
[$] time={4,4} | voices={S}
---

[S] |d :r !m :f ||
[1] do re mi fa
"#;

    assert_diagnostic_snapshot("undefined_verse_metdata_error", source);
}

#[test]
fn test_undefined_voice_parameter_error() {
    let source = r#"
[$] time={4,4}
---

[S] |d :r !m :f ||
"#;

    assert_diagnostic_snapshot("undefined_voice_parameter_error", source);
}

#[test]
fn test_undefined_time_parameter_error() {
    let source = r#"
[$] voices={S}
---

[S] |d :r !m :f ||
"#;

    assert_diagnostic_snapshot("undefined_time_parameter_error", source);
}

#[test]
fn test_non_top_level_override_error() {
    let source = r#"
[$] time={4,4} | voices={S,T}
---

[S] |d :r !m :f ||

++

[$] key={C}

[T] |d :r !m :f ||
"#;

    assert_diagnostic_snapshot("non_top_level_override_error", source);
}

#[test]
fn test_unused_lyrics_join_error() {
    let source = r#"
[#] verses={1}

[$] time={4,4} | voices={S}
---

[S] |d :r !m :f ||
[1] do re mi fa\ @@

"#;

    assert_diagnostic_snapshot("unused_lyrics_join_error", source);
}

#[test]
fn test_unmatched_timestamp_error() {
    let source = r#"
[$] time={4,4} | voices={S}
---

[$] dynamics={f:6}

[S] |d :r !m :f ||
"#;

    assert_diagnostic_snapshot("unmatched_timestamp_error", source);
}

#[test]
fn test_invalid_section_merge_error() {
    let source = r#"
[$] time={4,4} | voices={S,T}
---

[S] |d :r !m :f |s :l !t :d+1 ||
[T] |d :r !m :f |s :l !t :d+1 ||

<<

[S] |d :r !m :f ||
"#;

    assert_diagnostic_snapshot("ivalid_section_merge_error", source);
}

#[test]

fn test_unknown_directive_error() {
    let source = r#"
;; @invalid foo 
[$] time={4,4} | voices={S}
---
[S] |d :r !m :f ||
"#;

    assert_diagnostic_snapshot("unknown_directive_error", source);
}

use crate::{Formatter, Injection, tests::utils::assert_formatted_snapshot};
use voxalfa_core::validation::Validator;

mod utils;

#[test]
fn test_space_formatting() {
    let input = r#"
  [#] title=  "foo"
[#]  author={"bar",  "lorem"}  

[$] time={4, 4}    | voices={ S}

---

[S] |d :r !m :f ||

"#;

    assert_formatted_snapshot("test_space_formatting", input);
}

#[test]
fn test_line_formatting() {
    let input = r#"
        ;; @version 0.1.0-alpha.1
  [#] title=  "foo"
[#]  author={"bar",  "lorem"}  
    [#] verses={1}
[$] time={4, 4}    | voices={ S}
---



[S] |d :r !m :f ||
[1] do re mi fa

"#;

    assert_formatted_snapshot("test_line_formatting", input);
}

#[test]
fn test_space_distribution() {
    let input = r#"
[#] verses={1}
[$] time={4,4} | voices={S}

---

[S] |d.r :m.f+1 !s.l :t,d-1 ||
[1] do re mi fa so la ti dooo
"#;

    assert_formatted_snapshot("test_space_distribution", input);
}

#[test]
fn test_section_space_distribution() {
    let input = r#"
[#] verses={1}
[$] time={4,4} | voices={S}

---

[S] |d.r :m.f+1 !s.l :t,d-1 ||
[1] do re mi fa so la ti dooo\@@

--

[S] |d :r !m :f |`ss`:l !t :d ||
[1] do re m fa soso@2 `la` ti do
"#;

    assert_formatted_snapshot("test_section_space_distribution", input);
}

#[test]
fn test_expr_reordering() {
    let input = r#"
[$] time={4,4} | voices={S}
[#] verses={1}

;; @version 0.1.0-alpha.1

---

[$] jump={DC} | key={C}

[1] do re mi fa so la ti@2
[S] |d.r :m.f+1 !s.l :t.-||
"#;

    assert_formatted_snapshot("test_expr_reordering", input);
}

#[test]
fn test_wildcard_lyrics_formatting() {
    let input = r#"
[#] verses={2}

[$] time={4,4} | voices={S}

---

[S] |d :r !m :f ||
[*] do re_mi fa\@@

--

[S] |s :l !t :d+1 ||
[*] so la ti do
"#;

    assert_formatted_snapshot("test_wildcard_lyrics_formatting", input);
}

#[test]
fn test_comment_ranking() {
    let input = r#"
;; @version 0.1.0-alpha.1

[$] time={4,4} | voices={S}

; style
;; @page width={800} | height={1000}
;; @note-text font-size={12}

---
"#;

    let mut validator = Validator::init().unwrap();
    let output = validator.analyze(input);

    assert!(!output.has_error(), "{:?}", output.diagnostics);

    let formatted = Formatter::new(&output).format_to_string().unwrap();

    assert!(formatted.contains("; style\n;; @page"));
}

#[test]
fn test_metadata_swap() {
    let input = r#"
[#] title="foo" | verses={1}
[$] time={4,4} | voices={S}

---

[S] |d :r !m :f ||
[1] do re mi fa
"#;

    let mut validator = Validator::init().unwrap();
    let output = validator.analyze(input);

    assert!(!output.has_error(), "{:?}", output.diagnostics);

    let formatted = Formatter::new(&output)
        .inject(Injection::slots().metadata("verses", "{2}"))
        .unwrap();

    assert!(formatted.contains(r#"[#] title="foo" | verses={2}"#));
}

#[test]
fn test_metadata_addition() {
    let input = r#"
[#] title="foo"
[$] time={4,4} | voices={S}

---

[S] |d :r !m :f ||
"#;

    let mut validator = Validator::init().unwrap();
    let output = validator.analyze(input);

    assert!(!output.has_error(), "{:?}", output.diagnostics);

    let formatted = Formatter::new(&output)
        .inject(Injection::slots().metadata("verses", "{2}"))
        .unwrap();

    assert!(formatted.contains(r#"[#] verses={2}"#));
}

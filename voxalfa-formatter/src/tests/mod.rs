use crate::tests::utils::assert_formatted_snapshot;

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
        ;; @version 0.1.0-alpha
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

;; @version 0.1.0-alpha

---

[$] jump={DC} | key={C}

[1] do re mi fa so la ti@2
[S] |d.r :m.f+1 !s.l :t.-||
"#;

    assert_formatted_snapshot("test_expr_reordering", input);
}

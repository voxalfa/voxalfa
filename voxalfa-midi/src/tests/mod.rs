use crate::tests::utils::run_snapshot;

mod utils;

#[test]
fn test_c_major_scale_alto() {
    let source = r#"
[$] key={C} | time={4,4} | tempo={50} | voices={A}

---

[A] |d :r !m :f |s :l !t :d+1 ||"#;

    run_snapshot("test_c_major_scale_alto", source);
}

#[test]
fn test_simple_loop() {
    let source = r#"
[$] key={C} | time={4,4} | tempo={50} | voices={A}

---

[A] |d :r !m :f ||

--

[$] mark={S} | jump={DS}

[A] |s :l !t :d+1 ||"#;

    run_snapshot("test_simple_loop", source);
}

#[test]
fn test_repeated_loop() {
    let source = r#"
[$] key={C} | time={4,4} | tempo={50} | voices={A}

---

[$] jump={DC} | repeat={2}

[A] |d :r !m :f ||"#;

    run_snapshot("test_repeated_loop", source);
}

#[test]
fn test_dc_al_fine() {
    let source = r#"
[$] key={C} | time={4,4} | tempo={50} | voices={A}

---

[$] mark={F}

[A] |d :r !m :f ||

--

[$] jump={DCF}

[A] |s :l !t :d+1 ||"#;

    run_snapshot("test_ds_al_fine", source);
}

#[test]
fn test_dc_al_coda() {
    let source = r#"
[$] key={C} | time={4,4} | tempo={50} | voices={A}

---

[$] mark={TC}

[A] |d :r !m :f ||

--

[$] jump={DCC}

[A] |s :l !t :d+1 ||

--

[$] mark={C}

[A] |m :- !- :-  ||

"#;

    run_snapshot("test_dc_al_coda", source);
}

use super::{build_layout, marker};
use crate::{model::MarkerKind, renderer::systems::SystemRowKind};
use voxalfa_core::data_types::Voice;

#[test]
fn test_marker_stack() {
    let (fonts, layouts) = build_layout(
        r#"
[$] key={C} | time={4,4} | tempo={100} | voices={S}
---

[$] key={G} | dynamics={f}

[S] |d :r !m :f ||
"#,
    );

    let layout = &layouts[0];
    let key = marker(layout, MarkerKind::Key);
    let dynamic = marker(layout, MarkerKind::Dynamic);

    assert!(key.rect(&fonts).bottom <= dynamic.rect(&fonts).top);
}

#[test]
fn test_segno_stack() {
    let (fonts, layouts) = build_layout(
        r#"
[$] key={C} | time={4,4} | tempo={100} | voices={S}
---

[$] mark={S} | dynamics={f}

[S] |d :r !m :f ||
"#,
    );

    let layout = &layouts[0];
    let segno = marker(layout, MarkerKind::Mark);
    let dynamic = marker(layout, MarkerKind::Dynamic);

    assert!(!segno.rect(&fonts).intersects(&dynamic.rect(&fonts)));
}

#[test]
fn test_span_collision() {
    let (fonts, layouts) = build_layout(
        r#"
[$] key={C} | time={4,4} | tempo={100} | voices={S}
---

[$] touches={fermata:2} | dynamics={cresc:0..3}

[S] |d :r !m :- ||
"#,
    );

    let layout = &layouts[0];
    let fermata = marker(layout, MarkerKind::Touch);
    let hairpin = &layout.spans[0];

    assert!(
        hairpin.center_y < fermata.rect(&fonts).top,
        "hairpin {} fermata {:?}",
        hairpin.center_y,
        fermata.rect(&fonts)
    );
}

#[test]
fn test_subsection_span() {
    let (_, layouts) = build_layout(
        r#"
[#] verses={1}

[$] key={C} | time={4,4} | tempo={100} | voices={S,A}
---

[$] dynamics={f}

[S] |d :r !m :f ||

[1] do re mi fa

++

[$] dynamics={p, cresc:1..2, f:2}

[A] |d :r !m :f ||

[1] do re mi fa
"#,
    );

    let layout = &layouts[0];
    let hairpin = &layout.spans[0];

    let soprano = layout
        .rows
        .iter()
        .find(|row| matches!(row.kind, SystemRowKind::Voice(voice) if voice.role == Voice::S))
        .unwrap();

    let alto = layout
        .rows
        .iter()
        .find(|row| matches!(row.kind, SystemRowKind::Voice(voice) if voice.role == Voice::A))
        .unwrap();

    assert!(hairpin.center_y > soprano.y);
    assert!(hairpin.center_y < alto.y);
}

#[test]
fn test_span_endpoint() {
    let (fonts, layouts) = build_layout(
        r#"
[$] key={C} | time={4,4} | tempo={100} | voices={S}
---

[$] dynamics={p, cresc:0..3, f:3}

[S] |d :r !m :f ||
"#,
    );

    let layout = &layouts[0];
    let hairpin = &layout.spans[0];

    let forte = layout
        .markers
        .iter()
        .find(|marker| marker.kind == MarkerKind::Dynamic && marker.pulse == 3)
        .unwrap();

    let expected_y = fonts.label.dynamic.line_center(forte.y);

    assert!((hairpin.center_y - expected_y).abs() < 0.1);
    assert!(hairpin.end_x <= forte.horizontal_bounds().0);
}

#[test]
fn test_rit_jump() {
    let (fonts, layouts) = build_layout(
        r#"
[$] key={C} | time={4,4} | tempo={100} | voices={S}
---

[$] tempo={rit} | jump={DS}

[S] |d :r !m :f ||
"#,
    );

    let layout = &layouts[0];
    let rit = &layout.spans[0];
    let jump = layout
        .markers
        .iter()
        .find(|marker| marker.kind == MarkerKind::Jump)
        .unwrap();

    assert!(rit.end_x <= jump.horizontal_bounds().0);
    assert!((rit.center_y - fonts.label.dynamic.line_center(jump.y)).abs() < 0.1);
}

#[test]
fn test_volta_jump() {
    let (_, layouts) = build_layout(
        r#"
[$] key={C} | time={4,4} | tempo={100} | voices={S}
---

[$] ending={1} | jump={DS}

[S] |d :r !m :f ||

--

[$] ending={2} | jump={DCF}

[S] |d :r !m :f ||
"#,
    );

    let layout = &layouts[0];
    let volta = &layout.voltas[0];

    let jump = layout
        .markers
        .iter()
        .find(|marker| marker.kind == MarkerKind::Jump)
        .unwrap();

    assert!((volta.label_baseline - jump.y).abs() < 0.1);
    assert!(volta.line_y < volta.label_baseline);
}

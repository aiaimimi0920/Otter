//! 共享滚动快照的独立几何/布局身份oracle；不把滚动状态写入权威scene。
use super::*;
const SCENE: &[u8] = br#"{"id":"root","type":"scroll","layout":{"width":64,"height":64},"children":[{"id":"first","type":"input","layout":{"height":20,"minHeight":20},"props":{"value":"source"}},{"id":"gap","type":"view","layout":{"height":80,"minHeight":80}},{"id":"last","type":"button","layout":{"height":20,"minHeight":20}}]}"#;
fn p() -> Presentation {
    Presentation::new(
        Scene::parse(SCENE).unwrap(),
        crate::surface_viewport::style(),
        crate::surface_viewport::context([64.; 2]),
        [64.; 2],
    )
    .unwrap()
}
fn y(p: &Presentation, id: &str) -> f32 {
    p.geometry()
        .nodes()
        .iter()
        .find(|g| g.id() == id)
        .unwrap()
        .origin()[1]
}
#[test]
fn scroll_snapshot_is_cow_and_layout_identity_does_not_change_on_scroll_or_authoritative_value() {
    let mut a = p();
    let identity = a.layout_identity();
    let before = a.scroll_state();
    assert!(a.wheel([8., 8.], 1, 40., false).unwrap());
    let saved = a.scroll_state();
    assert_eq!(y(&a, "first"), -40.);
    let mut b = p();
    b.restore_scroll(&saved).unwrap();
    assert_eq!(y(&b, "last"), 60.);
    assert!(a.wheel([8., 8.], 1, 1000., false).unwrap());
    assert_eq!(y(&a, "last"), 44.);
    assert_eq!(
        y(&b, "last"),
        60.,
        "later writes cannot mutate a captured activation"
    );
    a.restore_scroll(&before).unwrap();
    assert_eq!(y(&a, "first"), 0.);
    assert!(a.layout_identity() == identity && b.layout_identity() == identity);
    let mut scene: serde_json::Value = serde_json::from_slice(SCENE).unwrap();
    scene["children"][0]["props"]["value"] = serde_json::json!("new authority");
    let next = Presentation::new(
        Scene::parse(&serde_json::to_vec(&scene).unwrap()).unwrap(),
        crate::surface_viewport::style(),
        crate::surface_viewport::context([64.; 2]),
        [64.; 2],
    )
    .unwrap();
    assert!(next.layout_identity() == identity);
    assert!(saved.bytes() >= 256);
}
#[test]
fn cropped_reveal_and_hidden_axis_reuse_scroll_without_granting_wheel_permission() {
    let mut a = p();
    assert!(a
        .reveal_focus_in(
            "last",
            crate::surface_clip::Rect {
                left: 0.,
                top: 32.,
                right: 64.,
                bottom: 64.
            }
        )
        .unwrap());
    assert_eq!(y(&a, "last"), 44.);
    a.restore_scroll(&Default::default()).unwrap();
    assert_eq!(y(&a, "last"), 100.);
    let bytes=br#"{"id":"root","type":"view","layout":{"height":64,"overflowY":"hidden"},"children":[{"id":"gap","type":"view","layout":{"height":100,"minHeight":100}},{"id":"last","type":"button","layout":{"height":20,"minHeight":20}}]}"#;
    let mut hidden = Presentation::new(
        Scene::parse(bytes).unwrap(),
        crate::surface_viewport::style(),
        crate::surface_viewport::context([64.; 2]),
        [64.; 2],
    )
    .unwrap();
    assert!(hidden.planned_wheel([8., 8.], 1, 40., false).is_none());
    assert!(hidden
        .reveal_focus_in(
            "last",
            crate::surface_clip::Rect {
                left: 0.,
                top: 0.,
                right: 64.,
                bottom: 64.
            }
        )
        .unwrap());
    assert_eq!(y(&hidden, "last"), 44.);
    assert!(hidden.planned_wheel([8., 48.], 1, -40., false).is_none());
}

//! Divider 固定有效属性、布局及原始输入边界。
use super::*;

#[test]
fn divider_has_full_parent_content_width_and_one_pixel_height() {
    let scene = Scene::parse(br#"{"id":"root","type":"column","layout":{"padding":4,"gap":3},"children":[{"id":"line","type":"divider"},{"id":"tail","type":"view","layout":{"height":10}}]}"#).unwrap();
    let presentation =
        crate::surface_presentation::Presentation::new(scene, base(), context(), [100., 100.])
            .unwrap();
    let nodes = presentation.geometry().nodes();
    assert_eq!(nodes[1].origin(), [4., 4.]);
    assert_eq!(nodes[1].size(), [92., 1.]);
    assert_eq!(nodes[2].origin()[1], 8.);
    assert!(presentation.text().items().is_empty());
}

#[test]
fn divider_ignores_custom_layout_and_style_like_frontend_hr() {
    let scene = Scene::parse(br#"{"id":"root","type":"divider","layout":{"width":0,"height":200,"position":"absolute","left":100},"style":{"opacity":0,"padding":100,"borderWidth":20,"background":"red"}}"#).unwrap();
    let presentation =
        crate::surface_presentation::Presentation::new(scene, base(), context(), [100., 100.])
            .unwrap();
    assert_eq!(presentation.geometry().nodes()[0].origin(), [0., 0.]);
    assert_eq!(presentation.geometry().nodes()[0].size(), [100., 1.]);
}

#[test]
fn divider_effective_view_does_not_bypass_raw_wire_limits_or_hide_children() {
    let style: serde_json::Map<String, serde_json::Value> = (0..129)
        .map(|i| (format!("x{i}"), serde_json::json!(0)))
        .collect();
    let json = serde_json::json!({"id":"root","type":"divider","style":style});
    assert!(matches!(
        Scene::parse(&serde_json::to_vec(&json).unwrap()),
        Err(crate::surface_scene::Error::Limit)
    ));
    let scene = Scene::parse(
        br#"{"id":"root","type":"divider","children":[{"id":"child","type":"image"}]}"#,
    )
    .unwrap();
    assert!(NativeMeasure::new(&scene, base(), context(), [100., 100.]).is_err());
}

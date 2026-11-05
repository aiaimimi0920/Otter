//! 文本命中、四向输出几何与字段组合同；不用被测映射函数计算预期矩形。
use super::*;
use crate::tile_surface_value::tests::{fixture, presentation};
use serde_json::json;
#[path = "tile_surface_text_wait_tests.rs"]
mod waits;
fn scene() -> serde_json::Value {
    json!({"id":"field","type":"input","layout":{"width":32,"height":16},"props":{"value":"old"},"events":{"input":"typing","change":"save"}})
}
#[test]
fn rotated_editor_bbox_uses_source_crop_once_and_does_not_require_a_handler() {
    let mut v = scene();
    v.as_object_mut().unwrap().remove("events");
    let f = fixture(v);
    for (rotation, pixel, expected) in [
        (crate::tile_wire::Rotation::Zero, [0, 0], [0, 0, 32, 16]),
        (crate::tile_wire::Rotation::Right, [0, 63], [0, 32, 16, 64]),
        (crate::tile_wire::Rotation::Half, [63, 63], [32, 48, 64, 64]),
        (crate::tile_wire::Rotation::Left, [63, 0], [48, 0, 64, 32]),
    ] {
        let mut layout = f.frame.layout.clone();
        layout.tiles[0].rotation = rotation;
        let frame = Frame {
            layout,
            size: crate::tile_wire::Size {
                width: 64,
                height: 64,
            },
            key: f.frame.key.clone(),
            images: f.frame.images.clone(),
            arts: f.frame.arts.clone(),
            live_until: None,
        };
        let field = hit(&frame, &f.art, &presentation(&f), pixel).unwrap();
        assert_eq!(field.rect, expected);
    }
}
#[test]
fn local_group_ignores_only_registered_field_values_and_never_changed_handlers_or_other_text() {
    let before = json!({"id":"root","type":"column","children":[scene(),{"id":"other","type":"input","props":{"value":"a"}},{"id":"label","type":"text","props":{"text":"fixed"}}]});
    let s = crate::surface_scene::Scene::parse(before.to_string().as_bytes()).unwrap();
    let hash = |id: &str| {
        crate::surface_edit_contract::fingerprint(s.root().visible_node(id).unwrap()).unwrap()
    };
    let first = crate::tile_surface_edit::Edit::new("field".into(), hash("field"));
    let second = crate::tile_surface_edit::Edit::peer("other".into(), hash("other"), &first);
    let mut after = before.clone();
    after["children"][0]["props"]["value"] = json!("new");
    after["children"][1]["props"]["value"] = json!("b");
    assert!(first.same_contract(&before, &after));
    assert!(second.same_group(&first));
    after["children"][0]["events"]["input"] = json!("other_action");
    assert!(!first.same_contract(&before, &after));
    after = before.clone();
    after["children"][2]["props"]["text"] = json!("changed");
    assert!(!second.same_contract(&before, &after));
    second.stop();
    assert!(!first.active());
}

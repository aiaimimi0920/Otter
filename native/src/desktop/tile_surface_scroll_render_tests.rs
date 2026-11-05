//! 重投影后真实DirectWrite嵌入值仍在；本地值/文本预览不因PaintFrame重建消失。
use super::*;
use serde_json::json;
fn scene() -> serde_json::Value {
    json!({"id":"root","type":"scroll","layout":{"width":64,"height":64},"children":[
        {"id":"input","type":"input","layout":{"height":20,"minHeight":20},"props":{"value":"source"}},
        {"id":"select","type":"select","layout":{"height":20,"minHeight":20},"props":{"value":"a","options":[{"value":"a","label":"Alpha"},{"value":"b","label":"Beta"}]}},
        {"id":"gap","type":"view","layout":{"height":80,"minHeight":80}},
        {"id":"last","type":"button","layout":{"height":20,"minHeight":20}}]})
}
fn text(p: &Prepared, id: &str) -> String {
    let crate::surface_embedded_control::Paint::Text { layout, .. } =
        p.presentation.embedded_control(id).unwrap()
    else {
        panic!()
    };
    layout.text_for_test()
}
#[test]
fn every_scroll_reembeds_text_and_local_select_and_planning_restores_the_original_cache() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let f = crate::tile_surface_value::tests::fixture(scene());
    let mut cache = BTreeMap::new();
    super::super::prepare(&mut cache, Some(&f.frame)).unwrap();
    let p = cache.get_mut("art").unwrap();
    p.text.insert("input".into(), "草稿🙂".into());
    p.local.insert(
        "select".into(),
        crate::tile_surface_local::Value::Select(Some(1)),
    );
    reembed(p).unwrap();
    let original = p.presentation.scroll_state();
    let planned = p
        .presentation
        .planned_wheel([8., 8.], 1, 40., false)
        .unwrap();
    assert!(install(&mut cache, &f.frame, "art", &planned).unwrap());
    assert_eq!(text(&cache["art"], "input"), "草稿🙂");
    assert_eq!(text(&cache["art"], "select"), "Beta");
    let current = cache["art"].presentation.scroll_state();
    let next = super::super::focus::next(&mut cache, &f.frame, None, true).unwrap();
    assert!(next.is_some());
    assert!(
        cache["art"].presentation.scroll_state() == current,
        "planning has no geometry side effect"
    );
    assert_eq!(text(&cache["art"], "input"), "草稿🙂");
    assert_eq!(text(&cache["art"], "select"), "Beta");
    assert!(install(&mut cache, &f.frame, "art", &original).unwrap());
    assert_eq!(text(&cache["art"], "input"), "草稿🙂");
}
#[test]
fn same_view_snapshot_replacement_keeps_scroll_but_content_shrink_clamps_and_new_generation_resets()
{
    let _com = crate::desktop::Apartment::new().unwrap();
    let f = crate::tile_surface_value::tests::fixture(scene());
    let mut cache = BTreeMap::new();
    super::super::prepare(&mut cache, Some(&f.frame)).unwrap();
    let planned = cache["art"]
        .presentation
        .planned_wheel([8., 8.], 1, 1000., false)
        .unwrap();
    install(&mut cache, &f.frame, "art", &planned).unwrap();
    for (gap, generation, expected) in [(80, 1, 44.), (10, 1, 44.), (80, 2, 120.)] {
        let mut next_scene = scene();
        next_scene["children"][2]["layout"]["height"] = json!(gap);
        next_scene["children"][2]["layout"]["minHeight"] = json!(gap);
        let next = crate::tile_surface_value::tests::fixture(next_scene);
        let art = Arc::new(Art {
            view: f.art.view.clone(),
            generation,
            snapshot: next.art.snapshot.clone(),
            size: f.art.size,
            images: Default::default(),
        });
        let frame = Frame {
            key: f.frame.key.clone(),
            layout: f.frame.layout.clone(),
            size: f.frame.size,
            images: Default::default(),
            arts: [("art".into(), art)].into(),
            live_until: None,
        };
        super::super::prepare(&mut cache, Some(&frame)).unwrap();
        assert_eq!(
            cache["art"]
                .presentation
                .geometry()
                .nodes()
                .iter()
                .find(|g| g.id() == "last")
                .unwrap()
                .origin()[1],
            expected
        );
    }
}

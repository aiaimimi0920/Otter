//! 真实 DirectWrite、图片比例与同一 PreparedScene 几何的 Grid 集成，不是浏览器 CSS 验收。
use super::*;
use crate::{surface_geometry::Geometry, surface_scene::Scene};
use serde_json::json;
fn parse(value: serde_json::Value) -> Scene {
    Scene::parse(&serde_json::to_vec(&value).unwrap()).unwrap()
}
#[test]
fn grid_native_wrap_reflows_auto_rows_and_hit_geometry_uses_the_same_cells() {
    let scene = parse(
        json!({"id":"root","type":"grid","layout":{"width":120,"gridTemplateColumns":"repeat(2,minmax(0,1fr))","gap":4},"children":[
            {"id":"long","type":"text","props":{"text":"wrapped words repeat across several native lines"}},
            {"id":"other","type":"text","props":{"text":"short"}},
            {"id":"tail","type":"button","props":{"label":"Tail"}}
        ]}),
    );
    let mut measure = NativeMeasure::new(&scene, base(), context(), [120., 300.]).unwrap();
    let prepared = measure.prepare(&scene, [120., 300.], context()).unwrap();
    assert!(prepared.passes() <= 8);
    let placed = prepared.tree().nodes();
    assert_eq!(placed[1].size[0], 58.);
    assert_eq!(placed[2].origin[0], 62.);
    assert!(prepared.text().items()[0].layout().line_count() >= 3);
    assert!(placed[3].origin[1] >= placed[1].size[1] + 3.99);
    let geometry = Geometry::from_tree(prepared.tree());
    let hits = crate::surface_hit_test::Hits::build(prepared.tree(), context(), |_| Some(16.));
    let hit = hits.hit(&geometry, [5., placed[3].origin[1] + 5.]).unwrap();
    assert_eq!(geometry.nodes()[hit].id(), "tail");
    let snapshot = crate::surface_layout_tree::Snapshot::new(prepared.tree());
    let restored = snapshot.restore(&scene).unwrap();
    assert_eq!(restored.nodes()[1].grid_cell, placed[1].grid_cell);
}
#[test]
fn grid_native_auto_column_uses_real_nowrap_width_not_equal_partition() {
    let scene = parse(
        json!({"id":"root","type":"grid","layout":{"width":300,"gridTemplateColumns":"minmax(0,1fr) auto","gap":10,"align":"center"},"children":[
            {"id":"flex","type":"text","props":{"text":"left"}},
            {"id":"status","type":"text","props":{"text":"Native status"},"style":{"whiteSpace":"nowrap"}}
        ]}),
    );
    let mut measure = NativeMeasure::new(&scene, base(), context(), [300., 100.]).unwrap();
    let prepared = measure.prepare(&scene, [300., 100.], context()).unwrap();
    let nodes = prepared.tree().nodes();
    let actual = prepared
        .text()
        .items()
        .iter()
        .find(|i| i.node_id() == "status")
        .unwrap()
        .layout()
        .size()[0];
    assert!((nodes[2].size[0] - actual).abs() < 0.01);
    assert!((nodes[1].size[0] + nodes[2].size[0] + 10. - 300.).abs() < 0.01);
}
#[test]
fn grid_native_image_percent_width_and_auto_ratio_use_cell_not_parent_column_fallback() {
    let scene = parse(
        json!({"id":"root","type":"grid","layout":{"width":300,"gridTemplateColumns":"100px minmax(0,1fr)"},"children":[
            {"id":"image","type":"image","layout":{"width":"50%"}},
            {"id":"text","type":"text","props":{"text":"T"}}
        ]}),
    );
    let mut measure = NativeMeasure::with_images(
        &scene,
        base(),
        context(),
        [300., 100.],
        [("image".to_owned(), [200, 100])].into(),
    )
    .unwrap();
    let prepared = measure.prepare(&scene, [300., 100.], context()).unwrap();
    let image = &prepared.tree().nodes()[1];
    assert_eq!(image.containing_width, 100.);
    assert_eq!(image.size, [50., 25.]);
    assert_eq!(image.grid_cell.unwrap()[0], 100.);
}
#[test]
fn grid_native_nested_measurements_are_cached_and_bounded() {
    let mut value = json!({"id":"text","type":"text","props":{"text":"bounded nested grid text"}});
    for i in 0..10 {
        value = json!({"id":format!("g{i}"),"type":"grid","layout":{"gridTemplateColumns":".75fr"},"children":[value]});
    }
    let scene = parse(value);
    let before = crate::surface_text::measure_calls();
    let mut measure = NativeMeasure::new(&scene, base(), context(), [400., 300.]).unwrap();
    let prepared = measure.prepare(&scene, [400., 300.], context()).unwrap();
    assert_eq!(prepared.tree().nodes().len(), 11);
    assert!(crate::surface_text::measure_calls() - before < 1000);
}
#[test]
fn grid_native_baseline_view_explicit_height_aligns_with_real_directwrite_baseline() {
    let scene = parse(
        json!({"id":"grid","type":"grid","layout":{"align":"baseline", "gridTemplateColumns":"40px 80px"},"children":[
            {"id":"box","type":"view","layout":{"height":30}},
            {"id":"text","type":"text","props":{"text":"Native"}}
        ]}),
    );
    let mut measure = NativeMeasure::new(&scene, base(), context(), [120., 100.]).unwrap();
    let prepared = measure.prepare(&scene, [120., 100.], context()).unwrap();
    let nodes = prepared.tree().nodes();
    assert_eq!(nodes[1].size[1], 30.);
    assert_eq!(nodes[1].origin[1], 0.);
    let layout = prepared.text().items()[0].layout();
    assert!((nodes[2].origin[1] + layout.baseline() - 30.).abs() < 0.01);
    assert!(nodes[0].size[1] >= 30. + layout.size()[1] - layout.baseline() - 0.01);
}

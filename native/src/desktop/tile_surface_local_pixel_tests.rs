//! 实际WIC/D2D像素：独立颜色/坐标oracle及既有权威Select基准，不截图用户屏幕。
use super::*;
use crate::tile_surface_local::{Cache, Value};
use serde_json::json;
use std::sync::Arc;
fn frame(kind: &str, value: serde_json::Value) -> Arc<Frame> {
    let f = crate::tile_surface_value::tests::fixture(
        json!({"id":"field","type":kind,"layout":{"width":64,"height":64},"props":{
        "value":value,"min":0,"max":10,"step":2,"options":[{"value":"a","label":"Alpha"},{"value":"b","label":"Beta"}]}}),
    );
    Arc::new(Frame {
        key: f.frame.key.clone(),
        layout: f.frame.layout.clone(),
        size: crate::tile_wire::Size {
            width: 64,
            height: 64,
        },
        images: f.frame.images.clone(),
        arts: f.frame.arts.clone(),
        live_until: None,
    })
}
fn rows(frame: &Frame, value: Value) -> std::rc::Rc<Vec<crate::tile_surface_local::Entry>> {
    let mut cache = BTreeMap::new();
    art::prepare(&mut cache, Some(frame)).unwrap();
    let focus = art::focus::at(&cache, frame, [10, 10]).unwrap();
    let mut c = Cache::default();
    c.set(focus, value, (0, 0)).unwrap();
    c.snapshot()
}
fn pixel(bytes: &[u8], x: usize, y: usize) -> &[u8] {
    &bytes[(y * 64 + x) * 4..(y * 64 + x + 1) * 4]
}
#[test]
fn local_switch_and_slider_draw_real_pixels_without_mutating_authoritative_scene() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let f = frame("switch", json!(false));
    let rows = rows(&f, Value::Switch(true));
    let baseline = unsafe { tests::render(&f) };
    let actual = unsafe { tests::render_local(&f, &rows) };
    // Switch按既有合同忽略layout高度，根自然高19；轨道中心在y=9而不是32。
    assert_eq!(pixel(&baseline, 32, 9), &[32, 23, 17, 255]);
    assert_eq!(pixel(&actual, 32, 9), &[56, 255, 217, 255]);
    assert_eq!(
        crate::surface_scene::Scene::parse(&f.arts["art"].snapshot.scene)
            .unwrap()
            .root()
            .props()["value"],
        json!(false)
    );
    let f = frame("slider", json!(2));
    let rows = self::rows(&f, Value::Slider(8.));
    let actual = unsafe { tests::render_local(&f, &rows) };
    assert_eq!(pixel(&actual, 48, 32), &[56, 255, 217, 255]);
    assert_eq!(pixel(&actual, 16, 32), &[159, 154, 146, 255]);
}
#[test]
fn select_preview_matches_existing_authoritative_label_path_and_new_source_rejects_old_preview() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let f = frame("select", json!("a"));
    let rows = rows(&f, Value::Select(Some(1)));
    let expected = unsafe { tests::render(&frame("select", json!("b"))) };
    let actual = unsafe { tests::render_local(&f, &rows) };
    assert_eq!(actual, expected);
    assert_ne!(actual, unsafe { tests::render(&f) });
    let next = frame("select", json!("missing"));
    assert_eq!(unsafe { tests::render_local(&next, &rows) }, unsafe {
        tests::render(&next)
    });
}

//! 同原生度量 owner 的独立预算/缓存诊断；不依赖 JS 异常文字或放宽产品时限。
use super::*;
use crate::surface_viewport::Viewport;
fn geometry() -> Geometry {
    let mut value = Geometry::default();
    value
        .begin(
            Some(Viewport::new([600, 400], 192).unwrap()),
            Instant::now() + std::time::Duration::from_millis(250),
        )
        .unwrap();
    value
}
fn source(width: u32) -> String {
    format!(r#"{{"id":"root","type":"column","layout":{{"width":{width},"height":30}}}}"#)
}
#[test]
fn javascript_geometry_native_cache_reuses_one_successful_layout_and_read_limit_is_terminal() {
    let mut value = geometry();
    let source = source(40);
    for _ in 0..MAX_READS {
        assert!(!value.read(&source, "root").is_empty());
    }
    assert_eq!(value.layouts, 1);
    assert_eq!(value.reads, MAX_READS);
    assert!(value.trace.len() <= 32);
    assert!(value.read(&source, "root").is_empty());
    assert!(!value.valid());
}
#[test]
fn javascript_geometry_native_changed_projection_has_bounded_remeasurements() {
    let mut value = geometry();
    for i in 0..MAX_LAYOUTS {
        assert!(!value.read(&source(40 + i as u32), "root").is_empty());
    }
    assert_eq!(value.layouts, MAX_LAYOUTS);
    assert!(value.read(&source(90), "root").is_empty());
    assert!(!value.valid());
}
#[test]
fn javascript_geometry_native_invalid_target_or_expired_work_cannot_be_caught_and_reused() {
    let mut value = geometry();
    assert!(value.read(&source(40), "unknown").is_empty());
    assert_eq!(value.trace.last().unwrap().0, "target-lookup");
    assert!(value.read(&source(40), "root").is_empty());
    value
        .begin(
            Some(Viewport::new([600, 400], 192).unwrap()),
            Instant::now(),
        )
        .unwrap();
    assert!(value.read(&source(40), "root").is_empty());
    assert_eq!(value.layouts, 0);
    assert_eq!(value.trace.last().unwrap().0, "read-admission");
}
#[test]
fn javascript_geometry_native_hidden_and_clipped_boxes_keep_border_box_semantics() {
    let mut value = geometry();
    let source = r#"{"id":"root","type":"column","layout":{"height":5},"children":[
      {"id":"visible","type":"column","layout":{"width":40,"height":30,"minHeight":30},"style":{"borderWidth":"2px"}},
      {"id":"hidden","type":"column","props":{"visible":false},"layout":{"height":30}}]}"#;
    let visible: serde_json::Value = serde_json::from_str(&value.read(source, "visible")).unwrap();
    assert_eq!(
        visible,
        serde_json::json!([0., 0., 40., 30., 36., 26., 36., 26., 0., 0., 72., 52.])
    );
    let hidden: serde_json::Value = serde_json::from_str(&value.read(source, "hidden")).unwrap();
    assert_eq!(
        hidden,
        serde_json::json!([0., 0., 0., 0., 0., 0., 0., 0., 0., 0., 0., 0.])
    );
}
#[test]
fn javascript_geometry_viewport_is_client_pixels_over_actual_dpi_and_rejects_bad_wire() {
    let view = Viewport::new([600, 400], 192).unwrap();
    assert_eq!(view.logical(), [300., 200.]);
    assert_eq!(view.ratio(), 2.);
    assert!(Viewport::new([32769, 1], 96).is_err());
    assert!(Viewport::new([1, 1], 0).is_err());
    let bad = serde_json::from_str::<Viewport>(r#"{"pixels":[1,1],"dpi":0}"#).unwrap();
    assert!(Geometry::default()
        .begin(Some(bad), Instant::now())
        .is_err());
    assert!(serde_json::from_str::<Viewport>(r#"{"pixels":[1,1],"dpi":96,"extra":true}"#).is_err());
}
#[test]
fn javascript_resize_geometry_content_box_reuses_containing_width_and_button_defaults() {
    let mut value = geometry();
    let source = r#"{"id":"root","type":"column","layout":{"width":200},"children":[
      {"id":"box","type":"column","layout":{"width":100,"height":80,"padding":"10%"},"style":{"borderWidth":"2px"}},
      {"id":"button","type":"button","layout":{"width":100,"height":40},"props":{"label":""}}]}"#;
    let content: Vec<f32> = serde_json::from_str(&value.read(source, "box")).unwrap();
    assert_eq!(&content[6..10], &[56., 36., 20., 20.]);
    let button: Vec<f32> = serde_json::from_str(&value.read(source, "button")).unwrap();
    // 默认按钮还具有 1px border，内容尺寸需同时扣除；padding 字段仍为 12/6。
    assert_eq!(&button[6..10], &[74., 26., 12., 6.]);
}

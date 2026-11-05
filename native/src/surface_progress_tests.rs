//! 进度值、原生布局与叶子边界，不把只读指示器当作输入控件。
use super::*;
#[test]
fn progress_values_are_finite_clamped_and_follow_numeric_defaults() {
    for (props, expected) in [
        ("{}", 0.),
        (r#"{"value":2,"max":4}"#, 0.5),
        (r#"{"value":9,"max":4}"#, 1.),
        (r#"{"value":-1}"#, 0.),
        (r#"{"value":0.5,"max":0}"#, 0.5),
        (r#"{"value":"1","max":-5}"#, 0.),
    ] {
        let json = format!(r#"{{"id":"root","type":"progress","props":{props}}}"#);
        let scene = Scene::parse(json.as_bytes()).unwrap();
        assert_eq!(crate::surface_progress::fraction(scene.root()), expected);
    }
}
#[test]
fn progress_uses_parent_font_height_full_width_and_no_text_layout() {
    let scene = Scene::parse(br#"{"id":"root","type":"column","style":{"fontSize":24},"children":[{"id":"progress","type":"progress","layout":{"width":1,"height":200},"props":{"value":0.5}}]}"#).unwrap();
    let presentation =
        crate::surface_presentation::Presentation::new(scene, base(), context(), [100., 100.])
            .unwrap();
    assert_eq!(presentation.geometry().nodes()[1].size(), [100., 24.]);
    assert!(presentation.text().items().is_empty());
}
#[test]
fn progress_rejects_hidden_child_runtime() {
    let scene = Scene::parse(
        br#"{"id":"root","type":"progress","children":[{"id":"child","type":"image"}]}"#,
    )
    .unwrap();
    assert!(NativeMeasure::new(&scene, base(), context(), [100., 100.]).is_err());
}

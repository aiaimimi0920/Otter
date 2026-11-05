use super::*;
use crate::surface_text::{Align, Whitespace};
#[path = "surface_selection_cross_node_tests.rs"]
mod cross_node;
#[path = "surface_selection_range_bench_tests.rs"]
mod range_bench;

fn style() -> Style<'static> {
    Style {
        family: "Segoe UI",
        size: 20.,
        weight: 400,
        whitespace: Whitespace::PreWrap,
        align: Align::Start,
        line_height: None,
    }
}
fn context() -> Context {
    Context {
        axis: 200.,
        viewport: [200., 100.],
        container: [200., 100.],
        root_font: 16.,
        font: 20.,
        fraction: None,
    }
}
fn scene() -> Scene {
    Scene::parse(br#"{"id":"root","type":"text","props":{"text":"Native selection","selectable":true},"layout":{"width":180,"height":40}}"#).unwrap()
}
fn owner() -> Presentation {
    Presentation::new(scene(), style(), context(), [200., 100.]).unwrap()
}
#[test]
fn text_selection_uses_one_owner_and_preserves_completed_range_until_clear() {
    let mut owner = owner();
    assert!(owner.begin_text_selection([0.1, 8.]).unwrap());
    assert!(owner.text_selection_dragging());
    assert_eq!(owner.selected_text().unwrap(), None);
    assert!(owner.update_text_selection([1000., 8.]).unwrap());
    assert_eq!(
        owner.selected_text().unwrap().as_deref(),
        Some("Native selection")
    );
    assert!(owner.finish_text_selection([1000., 8.]).unwrap());
    assert!(!owner.text_selection_dragging());
    owner.cancel_text_drag();
    assert!(owner.selected_text().unwrap().is_some());
    owner.clear_text_selection();
    assert_eq!(owner.selected_text().unwrap(), None);
}
#[test]
fn text_selection_cancel_replace_and_reflow_do_not_retarget_old_text() {
    let mut owner = owner();
    owner.begin_text_selection([0.1, 8.]).unwrap();
    owner.update_text_selection([1000., 8.]).unwrap();
    owner.cancel_text_drag();
    assert_eq!(owner.selected_text().unwrap(), None);
    owner.begin_text_selection([0.1, 8.]).unwrap();
    owner.finish_text_selection([1000., 8.]).unwrap();
    owner.reflow(style(), context(), [160., 100.]).unwrap();
    assert_eq!(
        owner.selected_text().unwrap().as_deref(),
        Some("Native selection")
    );
    let bad = Scene::parse(br#"{"id":"bad","type":"image"}"#).unwrap();
    assert!(owner
        .replace(bad, style(), context(), [160., 100.])
        .is_err());
    assert!(owner.selected_text().unwrap().is_some());
    owner
        .replace(scene(), style(), context(), [200., 100.])
        .unwrap();
    assert_eq!(owner.selected_text().unwrap(), None);
    assert!(!owner.update_text_selection([50., 8.]).unwrap());
}
#[test]
fn text_selection_cannot_start_through_cover_or_outside_visible_text() {
    let mut owner = owner();
    assert!(!owner.begin_text_selection([190., 8.]).unwrap());
    assert!(!owner.begin_text_selection([1., 80.]).unwrap());
    let covered = Scene::parse(br##"{"id":"root","type":"stack","layout":{"position":"relative","width":200,"height":100},"children":[{"id":"text","type":"text","props":{"text":"Text","selectable":true},"layout":{"width":180,"height":40}},{"id":"cover","type":"view","layout":{"position":"absolute","left":0,"top":0,"width":200,"height":100},"style":{"background":"#ff0000"}}]}"##).unwrap();
    owner
        .replace(covered, style(), context(), [200., 100.])
        .unwrap();
    assert_eq!(owner.hit_test([1., 8.]), Some("cover"));
    assert!(!owner.begin_text_selection([1., 8.]).unwrap());
}
#[test]
fn text_selection_reflow_clears_indices_when_normalized_content_changes() {
    let scene = Scene::parse(br#"{"id":"root","type":"text","props":{"text":"A  B","selectable":true},"layout":{"width":180,"height":40}}"#).unwrap();
    let mut owner = Presentation::new(scene, style(), context(), [200., 100.]).unwrap();
    owner.begin_text_selection([0.1, 8.]).unwrap();
    owner.finish_text_selection([1000., 8.]).unwrap();
    assert_eq!(owner.selected_text().unwrap().as_deref(), Some("A  B"));
    owner
        .reflow(
            Style {
                whitespace: Whitespace::Normal,
                ..style()
            },
            context(),
            [200., 100.],
        )
        .unwrap();
    assert_eq!(owner.selected_text().unwrap(), None);
}

#[path = "surface_selection_pixels_tests.rs"]
mod pixels;

#[test]
fn text_selection_keyboard_extends_reverses_collapses_without_changing_owner() {
    let mut owner = owner();
    assert!(!owner.step_text_selection("root", true, true).unwrap());
    owner.begin_text_selection([0.1, 8.]).unwrap();
    assert!(!owner.step_text_selection("root", true, true).unwrap());
    owner.finish_text_selection([0.1, 8.]).unwrap();
    assert!(!owner.step_text_selection("other", true, true).unwrap());
    assert!(owner.step_text_selection("root", true, true).unwrap());
    assert_eq!(owner.selected_text().unwrap().as_deref(), Some("N"));
    owner.step_text_selection("root", true, true).unwrap();
    assert_eq!(owner.selected_text().unwrap().as_deref(), Some("Na"));
    owner.step_text_selection("root", false, true).unwrap();
    assert_eq!(owner.selected_text().unwrap().as_deref(), Some("N"));
    owner.step_text_selection("root", true, false).unwrap();
    assert_eq!(owner.selected_text().unwrap(), None);
    owner.step_text_selection("root", true, true).unwrap();
    assert_eq!(owner.selected_text().unwrap().as_deref(), Some("a"));
    owner.step_text_selection("root", false, false).unwrap();
    owner.step_text_selection("root", false, true).unwrap();
    assert_eq!(owner.selected_text().unwrap().as_deref(), Some("N"));
    owner.clear_text_selection();
    assert!(!owner.step_text_selection("root", true, true).unwrap());
}

#[test]
fn text_selection_vertical_column_is_local_to_gesture_and_resets_on_reflow() {
    let mut owner = owner();
    assert!(!owner.vertical_text_selection("root", true).unwrap());
    owner.begin_text_selection([0.1, 8.]).unwrap();
    assert!(!owner.vertical_text_selection("root", true).unwrap());
    owner.finish_text_selection([0.1, 8.]).unwrap();
    assert!(owner.vertical_text_selection("root", true).unwrap());
    assert!(owner.selection.as_ref().unwrap().vertical_point.is_some());
    assert!(!owner.vertical_text_selection("other", true).unwrap());
    owner.step_text_selection("root", true, true).unwrap();
    assert!(owner.selection.as_ref().unwrap().vertical_point.is_none());
    owner.vertical_text_selection("root", true).unwrap();
    let selected = owner.selected_text().unwrap();
    owner.reflow(style(), context(), [160., 100.]).unwrap();
    assert!(owner.selection.as_ref().unwrap().vertical_point.is_none());
    assert_eq!(owner.selected_text().unwrap(), selected);
}

#[test]
fn text_selection_vertical_first_move_preserves_mouse_soft_wrap_line() {
    let scene = Scene::parse(br#"{"id":"root","type":"text","props":{"text":"one two three four five six seven eight","selectable":true},"layout":{"width":70,"height":300}}"#).unwrap();
    let mut owner = Presentation::new(scene, style(), context(), [200., 100.]).unwrap();
    let line_height = owner.text.items()[0]
        .layout()
        .cluster_rect(0)
        .unwrap()
        .bottom;
    assert!(owner.begin_text_selection([0.1, 8.]).unwrap());
    owner.finish_text_selection([300., 8.]).unwrap();
    assert!(owner.selection.as_ref().unwrap().vertical_point.unwrap()[1] < line_height);
    owner.vertical_text_selection("root", true).unwrap();
    let point = owner.selection.as_ref().unwrap().vertical_point.unwrap();
    assert!(point[1] >= line_height && point[1] < line_height * 2.);
}

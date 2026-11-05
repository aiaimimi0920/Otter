use super::*;
#[path = "surface_border_pixels_tests.rs"]
mod border_pixels;
#[path = "surface_border_sides_tests.rs"]
mod border_sides;
#[path = "surface_box_tests.rs"]
mod box_model;
#[path = "surface_button_tests.rs"]
mod button;
#[path = "surface_divider_tests.rs"]
mod divider;
#[path = "surface_text_frame_tests.rs"]
mod frame;
#[path = "surface_grid_native_tests.rs"]
mod grid;
#[path = "surface_hit_tests.rs"]
mod hit_tests;
#[path = "surface_icon_tests.rs"]
mod icon;
#[path = "surface_icon_pixels_tests.rs"]
mod icon_pixels;
#[path = "surface_image_measure_tests.rs"]
mod image_measure;
#[path = "surface_padding_tests.rs"]
mod padding;
#[path = "surface_paint_pixels_tests.rs"]
mod paint_pixels;
#[path = "surface_text_pixels_tests.rs"]
mod pixels;
#[path = "surface_presentation_tests.rs"]
mod presentation;
#[path = "surface_progress_tests.rs"]
mod progress;
#[path = "surface_reflow_tests.rs"]
mod reflow;
#[path = "surface_spacer_tests.rs"]
mod spacer;
#[path = "surface_switch_tests.rs"]
mod switch;
#[path = "surface_switch_visual_tests.rs"]
mod switch_visual;
#[path = "surface_visibility_tests.rs"]
mod visibility;
#[test]
fn tree_geometry_uses_each_nodes_resolved_font_without_reapplying_em() {
    let scene=Scene::parse(br#"{"id":"root","type":"row","style":{"fontSize":"2em"},"layout":{"padding":"1em"},"children":[{"id":"text","type":"text","props":{"text":"A"},"style":{"fontSize":"2em"},"layout":{"width":"1em","height":"1em","position":"relative","left":"0.5em"}}]}"#).unwrap();
    let mut measure = NativeMeasure::new(&scene, base(), context(), [400., 300.]).unwrap();
    let tree = Tree::build(&scene, [400., 300.], context(), &mut measure).unwrap();
    assert_eq!(tree.nodes()[1].origin, [64., 32.]);
    assert_eq!(tree.nodes()[1].size, [64., 64.]);
}
use crate::{
    surface_layout_tree::Tree,
    surface_text::{Align, Whitespace},
};
fn base() -> Style<'static> {
    Style {
        family: "Segoe UI",
        size: 16.,
        weight: 400,
        whitespace: Whitespace::Normal,
        align: Align::Start,
        line_height: None,
    }
}
fn context() -> Context {
    Context {
        axis: 400.,
        viewport: [800., 600.],
        container: [400., 300.],
        root_font: 16.,
        font: 16.,
        fraction: None,
    }
}
#[test]
fn real_native_measurement_drives_nested_scene_tree() {
    let scene=Scene::parse(br#"{"id":"root","type":"column","layout":{"gap":5},"children":[{"id":"row","type":"row","layout":{"padding":3},"children":[{"id":"text","type":"text","props":{"text":"Native text"},"style":{"fontSize":"2em","whiteSpace":"nowrap"}}]},{"id":"tail","type":"text","props":{"text":"Tail"}}]}"#).unwrap();
    let mut measure = NativeMeasure::new(&scene, base(), context(), [400., 300.]).unwrap();
    let tree = Tree::build(&scene, [400., 300.], context(), &mut measure).unwrap();
    assert_eq!(tree.nodes().len(), 4);
    let row = &tree.nodes()[1];
    let text = &tree.nodes()[2];
    let tail = &tree.nodes()[3];
    assert_eq!(text.origin, [3., 3.]);
    assert!(row.size[1] > 30.);
    assert!(text.size[0] > 0.);
    assert!((tail.origin[1] - row.size[1] - 5.).abs() < 0.01);
}
#[test]
fn native_measure_rejects_foreign_scene_and_unimplemented_leaf() {
    let scene = Scene::parse(br#"{"id":"root","type":"text","props":{"text":"a"}}"#).unwrap();
    let other = Scene::parse(br#"{"id":"root","type":"text","props":{"text":"b"}}"#).unwrap();
    let mut measure = NativeMeasure::new(&scene, base(), context(), [400., 300.]).unwrap();
    assert!(measure
        .measure(other.root(), [400., 300.], context())
        .is_err());
    let image = Scene::parse(br#"{"id":"image","type":"image"}"#).unwrap();
    assert!(NativeMeasure::new(&image, base(), context(), [400., 300.]).is_err());
}

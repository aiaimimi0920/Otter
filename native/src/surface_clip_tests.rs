use super::*;
use crate::{
    surface_layout_tree::{Intrinsic, Measure},
    surface_length::Context,
    surface_scene::{Node, Scene},
};
struct MeasureBoxes;
impl Measure for MeasureBoxes {
    type Error = ();
    fn measure(&mut self, _: &Node, _: [f32; 2], _: Context) -> Result<Intrinsic, ()> {
        Ok(Intrinsic {
            size: [20., 20.],
            baseline: None,
        })
    }
}
fn context() -> Context {
    Context {
        axis: 100.,
        viewport: [100., 100.],
        container: [100., 100.],
        font: 16.,
        root_font: 16.,
        fraction: None,
    }
}
#[test]
fn ancestor_clips_normalize_other_axis_and_keep_root_boundary() {
    let scene=Scene::parse(br#"{"id":"root","type":"column","layout":{"height":100},"children":[{"id":"box","type":"column","layout":{"width":40,"height":40,"overflowX":"hidden"},"children":[{"id":"text","type":"text","layout":{"width":80,"height":80}}]}]}"#).unwrap();
    let tree = Tree::build(&scene, [100., 100.], context(), &mut MeasureBoxes).unwrap();
    let clips = ancestors(&tree);
    assert_eq!(
        clips[2],
        Rect {
            left: 0.,
            top: 0.,
            right: 40.,
            bottom: 40.
        }
    );
    assert_eq!(tree.nodes()[2].size[0], 80.);
}
#[test]
fn scroll_clips_both_axes_without_changing_geometry() {
    let scene=Scene::parse(br#"{"id":"root","type":"scroll","layout":{"padding":10},"children":[{"id":"text","type":"text","layout":{"position":"relative","left":100,"width":20}}]}"#).unwrap();
    let tree = Tree::build(&scene, [100., 100.], context(), &mut MeasureBoxes).unwrap();
    let clips = ancestors(&tree);
    let child = &tree.nodes()[1];
    assert!(clips[1]
        .intersect(Rect::from_box(child.origin, child.size))
        .empty());
    assert_eq!(child.origin[0], 110.);
}

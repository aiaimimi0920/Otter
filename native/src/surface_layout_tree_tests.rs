use super::*;
fn context() -> Context {
    Context {
        axis: 100.,
        viewport: [100., 100.],
        container: [100., 100.],
        root_font: 16.,
        font: 16.,
        fraction: None,
    }
}
struct Measurements {
    calls: Vec<String>,
    fail: bool,
}
impl Measure for Measurements {
    type Error = ();
    fn measure(&mut self, node: &Node, _: [f32; 2], _: Context) -> Result<Intrinsic, ()> {
        self.calls.push(node.id().to_owned());
        if self.fail {
            return Err(());
        }
        Ok(Intrinsic {
            size: [20., 20.],
            baseline: Some(15.),
        })
    }
}
#[test]
fn nested_real_scene_accumulates_origins_and_preserves_parent_order() {
    let scene=Scene::parse(br#"{"id":"root","type":"column","layout":{"padding":10,"height":100},"children":[{"id":"row","type":"row","layout":{"padding":5,"height":40},"children":[{"id":"a","type":"text","layout":{"width":20}},{"id":"b","type":"text","layout":{"width":20}}]}]}"#).unwrap();
    let mut measure = Measurements {
        calls: vec![],
        fail: false,
    };
    let tree = Tree::build(&scene, [100., 100.], context(), &mut measure).unwrap();
    let nodes = tree.nodes();
    assert_eq!(
        nodes.iter().map(|p| p.node.id()).collect::<Vec<_>>(),
        ["root", "row", "a", "b"]
    );
    assert_eq!(nodes[1].origin, [10., 10.]);
    assert_eq!(nodes[1].size, [80., 40.]);
    assert_eq!(nodes[2].origin, [15., 15.]);
    assert_eq!(nodes[3].origin, [35., 15.]);
    assert_eq!(nodes[2].parent, Some(1));
    assert_eq!(nodes[1].parent, Some(0));
    assert_eq!(measure.calls, ["root", "row", "a", "b"]);
}
#[test]
fn failed_measurement_does_not_return_partial_tree_and_can_be_retried() {
    let scene =
        Scene::parse(br#"{"id":"root","type":"stack","children":[{"id":"a","type":"text"}]}"#)
            .unwrap();
    let mut measure = Measurements {
        calls: vec![],
        fail: true,
    };
    assert!(matches!(
        Tree::build(&scene, [100., 100.], context(), &mut measure),
        Err(Error::Measurement)
    ));
    measure.fail = false;
    assert_eq!(
        Tree::build(&scene, [100., 100.], context(), &mut measure)
            .unwrap()
            .nodes()
            .len(),
        2
    );
    assert!(matches!(
        Tree::build(&scene, [f32::NAN, 100.], context(), &mut measure),
        Err(Error::Input)
    ));
}

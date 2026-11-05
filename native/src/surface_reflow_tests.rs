use super::*;
#[test]
fn wrapped_text_height_propagates_through_auto_container_to_following_sibling() {
    let scene=Scene::parse(br#"{"id":"root","type":"column","layout":{"gap":5},"children":[{"id":"row","type":"row","children":[{"id":"a","type":"text","props":{"text":"native text wrapping native text wrapping"},"layout":{"width":200}},{"id":"b","type":"text","props":{"text":"native text wrapping native text wrapping"},"layout":{"width":200}}]},{"id":"tail","type":"text","props":{"text":"Tail"}}]}"#).unwrap();
    let mut owner = NativeMeasure::new(&scene, base(), context(), [100., 1000.]).unwrap();
    let initial = Tree::build(&scene, [100., 1000.], context(), &mut owner).unwrap();
    let old_tail = initial
        .nodes()
        .iter()
        .find(|p| p.node.id() == "tail")
        .unwrap()
        .origin[1];
    let prepared = owner.prepare(&scene, [100., 1000.], context()).unwrap();
    assert!(prepared.passes() >= 2 && prepared.passes() <= 8);
    let row = prepared
        .tree()
        .nodes()
        .iter()
        .find(|p| p.node.id() == "row")
        .unwrap();
    let tail = prepared
        .tree()
        .nodes()
        .iter()
        .find(|p| p.node.id() == "tail")
        .unwrap();
    let text = prepared
        .text()
        .items()
        .iter()
        .find(|p| p.node_id() == "a")
        .unwrap();
    assert!(tail.origin[1] > old_tail);
    assert!((row.size[1] - text.layout().size()[1]).abs() < 0.01);
    assert!((tail.origin[1] - row.size[1] - 5.).abs() < 0.01);
    let again = owner.prepare(&scene, [100., 1000.], context()).unwrap();
    assert_eq!(again.tree().nodes().last().unwrap().origin, tail.origin);
}
#[test]
fn empty_scene_prepares_without_feedback_loop() {
    let scene = Scene::parse(br#"{"id":"root","type":"column"}"#).unwrap();
    let mut owner = NativeMeasure::new(&scene, base(), context(), [100., 100.]).unwrap();
    assert_eq!(
        owner
            .prepare(&scene, [100., 100.], context())
            .unwrap()
            .passes(),
        1
    );
}

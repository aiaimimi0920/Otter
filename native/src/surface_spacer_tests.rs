//! Spacer 的真实测量、伸缩和空白叶子合同；不引入新的绘制资源或输入能力。
use super::*;

#[test]
fn spacers_share_remaining_space_in_both_flow_directions() {
    for kind in ["row", "column"] {
        let json = format!(
            r#"{{"id":"root","type":"{kind}","layout":{{"width":100,"height":100}},"children":[{{"id":"head","type":"view","layout":{{"width":20,"height":20}}}},{{"id":"first","type":"spacer"}},{{"id":"second","type":"spacer"}},{{"id":"tail","type":"view","layout":{{"width":20,"height":20}}}}]}}"#
        );
        let scene = Scene::parse(json.as_bytes()).unwrap();
        let presentation =
            crate::surface_presentation::Presentation::new(scene, base(), context(), [100., 100.])
                .unwrap();
        let nodes = presentation.geometry().nodes();
        let axis = usize::from(kind == "column");
        assert_eq!(nodes[2].size()[axis], 30.);
        assert_eq!(nodes[3].size()[axis], 30.);
        assert_eq!(nodes[4].origin()[axis], 80.);
        assert!(presentation.text().items().is_empty());
    }
}

#[test]
fn spacer_explicit_zero_grow_preserves_its_basis_and_insets() {
    let scene = Scene::parse(br#"{"id":"root","type":"row","layout":{"height":50},"children":[{"id":"space","type":"spacer","layout":{"width":20,"grow":0,"padding":2},"style":{"borderWidth":1}},{"id":"flex","type":"spacer"}]}"#).unwrap();
    let presentation =
        crate::surface_presentation::Presentation::new(scene, base(), context(), [100., 50.])
            .unwrap();
    assert_eq!(presentation.geometry().nodes()[1].size()[0], 20.);
    assert_eq!(presentation.geometry().nodes()[2].size()[0], 80.);
}

#[test]
fn spacer_is_leaf_and_cannot_hide_an_unimplemented_subtree() {
    let scene = Scene::parse(
        br#"{"id":"root","type":"spacer","children":[{"id":"hidden","type":"image"}]}"#,
    )
    .unwrap();
    assert!(NativeMeasure::new(&scene, base(), context(), [100., 100.]).is_err());
}

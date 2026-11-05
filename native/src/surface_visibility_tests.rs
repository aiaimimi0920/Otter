//! visible=false 与 opacity=0 不同：隐藏子树不测量、不占位、不参与最终快照。
use super::*;
#[test]
fn hidden_subtree_is_not_measured_and_does_not_add_flow_gap() {
    let scene = Scene::parse(br#"{"id":"root","type":"column","layout":{"gap":5},"children":[{"id":"first","type":"view","layout":{"height":10}},{"id":"hidden","type":"column","props":{"visible":false},"layout":{"height":100},"children":[{"id":"unsupported","type":"image"}]},{"id":"last","type":"view","layout":{"height":20}}]}"#).unwrap();
    let presentation =
        crate::surface_presentation::Presentation::new(scene, base(), context(), [100., 100.])
            .unwrap();
    let nodes = presentation.geometry().nodes();
    assert_eq!(
        nodes.iter().map(|node| node.id()).collect::<Vec<_>>(),
        ["root", "first", "last"]
    );
    assert_eq!(nodes[2].origin()[1], 15.);
    assert_eq!(nodes[0].size()[1], 35.);
}
#[test]
fn hidden_root_replacement_clears_geometry_and_text_and_can_be_shown_again() {
    let hidden =
        || Scene::parse(br#"{"id":"root","type":"image","props":{"visible":false}}"#).unwrap();
    let visible =
        || Scene::parse(br#"{"id":"root","type":"text","props":{"text":"shown"}}"#).unwrap();
    let mut presentation =
        crate::surface_presentation::Presentation::new(visible(), base(), context(), [100., 100.])
            .unwrap();
    presentation
        .replace(hidden(), base(), context(), [100., 100.])
        .unwrap();
    assert!(presentation.geometry().nodes().is_empty());
    assert!(presentation.text().items().is_empty());
    presentation
        .replace(visible(), base(), context(), [100., 100.])
        .unwrap();
    assert_eq!(presentation.text().items().len(), 1);
}
#[test]
fn only_boolean_false_hides_nodes_and_raw_identity_checks_still_apply() {
    for value in ["true", "null", "0", r#""false""#] {
        let json = format!(r#"{{"id":"root","type":"image","props":{{"visible":{value}}}}}"#);
        let scene = Scene::parse(json.as_bytes()).unwrap();
        assert!(NativeMeasure::new(&scene, base(), context(), [100., 100.]).is_err());
    }
    assert!(Scene::parse(br#"{"id":"root","type":"view","props":{"visible":false},"children":[{"id":"root","type":"view"}]}"#).is_err());
}

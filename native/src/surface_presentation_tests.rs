//! 独立持有布局的窗口呈现状态，失败更新不得替换最后有效场景。
use super::*;
use crate::surface_presentation::Presentation;

fn scene(id: &str) -> Scene {
    Scene::parse(format!(r#"{{"id":"{id}","type":"text","props":{{"text":"native text wrapping native text wrapping"}}}}"#).as_bytes()).unwrap()
}
#[test]
fn owned_text_survives_scene_and_measure_owner_drop() {
    let frame = {
        let scene = scene("original");
        let mut measure = NativeMeasure::new(&scene, base(), context(), [100., 80.]).unwrap();
        measure
            .prepare(&scene, [100., 80.], context())
            .unwrap()
            .into_text()
    };
    assert_eq!(frame.items()[0].node_id(), "original");
    assert!(frame.items()[0].layout().line_count() > 1);
}
#[test]
fn presentation_replaces_and_reflows_only_after_successful_preparation() {
    let mut owner = Presentation::new(scene("first"), base(), context(), [100., 80.]).unwrap();
    let unsupported = Scene::parse(br#"{"id":"bad","type":"image"}"#).unwrap();
    assert!(owner
        .replace(unsupported, base(), context(), [50., 80.])
        .is_err());
    assert_eq!(owner.scene().root().id(), "first");
    assert_eq!(owner.text().items()[0].node_id(), "first");
    assert_eq!(owner.size(), [100., 80.]);
    assert_eq!(owner.geometry().nodes()[0].id(), "first");
    let initial_height = owner.text().items()[0].layout().size()[1];
    assert_eq!(owner.geometry().nodes()[0].size(), [100., initial_height]);
    assert!(owner.reflow(base(), context(), [f32::NAN, 80.]).is_err());
    assert_eq!(owner.size(), [100., 80.]);
    owner.reflow(base(), context(), [50., 80.]).unwrap();
    let wrapped_height = owner.text().items()[0].layout().size()[1];
    assert!(wrapped_height > initial_height);
    assert_eq!(owner.text().items()[0].allocated(), [50., wrapped_height]);
    assert_eq!(owner.geometry().nodes()[0].size(), [50., wrapped_height]);
    assert_eq!(owner.text().items()[0].clip().bottom, 80.);
    owner
        .replace(scene("second"), base(), context(), [200., 80.])
        .unwrap();
    assert_eq!(owner.text().items()[0].node_id(), "second");
    assert_eq!(owner.scene().root().id(), "second");
    assert_eq!(owner.geometry().nodes()[0].id(), "second");
}

#[test]
fn presentation_keeps_container_geometry_and_clipped_children_consistent_with_text() {
    let scene = Scene::parse(br#"{"id":"root","type":"column","children":[{"id":"box","type":"column","layout":{"height":40,"overflowX":"hidden"},"children":[{"id":"text","type":"text","props":{"text":"Text"},"layout":{"width":50,"height":20,"position":"relative","left":200}}]}]}"#).unwrap();
    let owner = Presentation::new(scene, base(), context(), [100., 80.]).unwrap();
    let nodes = owner.geometry().nodes();
    assert_eq!(nodes.len(), 3);
    assert_eq!(nodes[0].parent(), None);
    assert_eq!(nodes[1].parent(), Some(0));
    assert_eq!(nodes[2].parent(), Some(1));
    assert_eq!(nodes[1].kind(), crate::surface_scene::Kind::Column);
    let text = &owner.text().items()[0];
    assert_eq!(nodes[2].id(), text.node_id());
    assert_eq!(nodes[2].origin(), text.origin());
    assert_eq!(nodes[2].size(), text.allocated());
    assert_eq!(nodes[2].clip(), text.clip());
    assert!(nodes[2].clip().empty());
}

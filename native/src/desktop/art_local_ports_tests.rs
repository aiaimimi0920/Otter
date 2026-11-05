//! 显式空集合、仅自建边可撤回及冲突原子性；不持像素或调用窗口。
use super::*;
use crate::art_execution_grant::package::ports::{Port, Ports};
use crate::workspace::Link;
fn id(value: &str) -> UnitId {
    UnitId::parse(value).unwrap()
}
fn ports(name: Option<&str>) -> Ports {
    Ports {
        inputs: name
            .into_iter()
            .map(|name| Port {
                name: name.into(),
                kind: "image".into(),
            })
            .collect(),
        outputs: vec![],
    }
}
fn graph() -> (Workspace, Link) {
    let mut graph = Workspace::with_initial(id("source"));
    graph.add_units([id("target")]).unwrap();
    let link = Link {
        id: id("edge"),
        from_unit: id("source"),
        from_port: "output".into(),
        to_unit: id("target"),
        to_port: "input_image".into(),
    };
    graph.connect(link.clone()).unwrap();
    (graph, link)
}
#[test]
fn explicit_empty_ports_retract_only_exact_owned_provisional_link() {
    let (graph, link) = graph();
    assert!(
        topology(&graph, &id("target"), true, Some(&link), &ports(None))
            .unwrap()
            .links()
            .is_empty()
    );
    assert_eq!(graph.links(), std::slice::from_ref(&link));
    assert_eq!(
        topology(&graph, &id("target"), true, None, &ports(None))
            .unwrap()
            .links(),
        graph.links()
    );
    let mut changed = link.clone();
    changed.from_port = "other".into();
    assert_eq!(
        topology(&graph, &id("target"), true, Some(&changed), &ports(None))
            .unwrap()
            .links(),
        graph.links()
    );
    assert_eq!(
        topology(&graph, &id("target"), false, Some(&link), &ports(None))
            .unwrap()
            .links(),
        graph.links()
    );
}
#[test]
fn custom_port_migration_preserves_identity_and_conflict_keeps_original_graph() {
    let (mut graph, link) = graph();
    let next = topology(
        &graph,
        &id("target"),
        true,
        Some(&link),
        &ports(Some("原始图片")),
    )
    .unwrap();
    assert_eq!(next.links()[0].id, link.id);
    assert_eq!(next.links()[0].from_unit, link.from_unit);
    assert_eq!(next.links()[0].to_port, "原始图片");
    let mut occupied = link.clone();
    occupied.id = id("occupied");
    occupied.to_port = "原始图片".into();
    graph.add_link(occupied).unwrap();
    let original = graph.links().to_vec();
    assert!(topology(
        &graph,
        &id("target"),
        true,
        Some(&link),
        &ports(Some("原始图片"))
    )
    .is_err());
    assert_eq!(graph.links(), original);
}
#[test]
fn missing_proof_is_legacy_but_signed_empty_collections_stay_empty() {
    let config = crate::desktop::document_state::local_art::Config::from("vendor/demo".to_owned());
    assert_eq!(
        config.inputs(),
        vec![("input_image".into(), "image".into())]
    );
    assert_eq!(config.primary_output().as_deref(), Some("output_image"));
    let mut record = Record {
        origin: "https://loom.example".into(),
        art_id: "vendor/demo".into(),
        art_version: "1.0.0".into(),
        package_digest: "a".repeat(64),
        sdk_version: "otter.art.v1".into(),
        ports: ports(None),
        visibility: None,
    };
    *config.ports.borrow_mut() = Some(Arc::new(record.clone()));
    assert!(config.inputs().is_empty());
    assert!(config.outputs().is_empty());
    assert!(config.primary_output().is_none());
    record.ports.outputs.push(Port {
        name: "rendered".into(),
        kind: "image".into(),
    });
    *config.ports.borrow_mut() = Some(Arc::new(record));
    assert_eq!(config.primary_output().as_deref(), Some("rendered"));
}

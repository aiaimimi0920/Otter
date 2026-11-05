//! 依赖子图边界：方向、导入环、分叉和最长受支持链。
use super::*;
fn id(value: &str) -> UnitId {
    UnitId::parse(value).unwrap()
}
fn edge(workspace: &mut Workspace, name: &str, from: &str, to: &str) {
    workspace
        .add_link(Link {
            id: id(name),
            from_unit: id(from),
            to_unit: id(to),
            from_port: "output".into(),
            to_port: "image".into(),
        })
        .unwrap();
}
#[test]
fn upstream_excludes_downstream_and_unrelated_but_retains_cycles_and_branches() {
    let mut workspace = Workspace::default();
    workspace
        .add_units(["target", "relay", "source", "other", "downstream"].map(id))
        .unwrap();
    edge(&mut workspace, "a", "relay", "target");
    edge(&mut workspace, "b", "source", "relay");
    edge(&mut workspace, "c", "source", "target");
    edge(&mut workspace, "cycle", "target", "relay");
    edge(&mut workspace, "down", "target", "downstream");
    assert_eq!(
        workspace.upstream_units(&id("target")).unwrap(),
        vec![&id("target"), &id("relay"), &id("source")]
    );
    assert_eq!(
        workspace.upstream_units(&id("other")).unwrap(),
        vec![&id("other")]
    );
    assert_eq!(
        workspace.upstream_units(&id("missing")),
        Err(Error::MissingUnit)
    );
}
#[test]
fn upstream_is_bounded_for_full_capacity_chain_and_imported_cycle() {
    let mut workspace = Workspace::default();
    workspace
        .add_units((0..MAX_UNITS).map(|i| id(&i.to_string())))
        .unwrap();
    for i in 1..MAX_UNITS {
        edge(
            &mut workspace,
            &format!("e{i}"),
            &(i - 1).to_string(),
            &i.to_string(),
        );
    }
    edge(&mut workspace, "cycle", &(MAX_UNITS - 1).to_string(), "0");
    let found = workspace
        .upstream_units(&id(&(MAX_UNITS - 1).to_string()))
        .unwrap();
    assert_eq!(found.len(), MAX_UNITS);
    for unit in workspace.units() {
        assert!(found.contains(&unit));
    }
}

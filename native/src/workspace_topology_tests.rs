use super::*;
fn id(name: &str) -> UnitId {
    UnitId::parse(name).unwrap()
}
fn link(name: &str, from: &str, to: &str) -> Link {
    Link {
        id: id(name),
        from_unit: id(from),
        from_port: "output_image".into(),
        to_unit: id(to),
        to_port: "image".into(),
    }
}
#[test]
fn topology_changes_for_successful_structure_edits_not_failed_or_noop_changes() {
    let mut workspace = Workspace::default();
    let empty = workspace.topology_stamp();
    assert!(!empty.matches(&Workspace::default()));
    workspace.clear();
    workspace.add_units([]).unwrap();
    assert!(empty.matches(&workspace));
    workspace.add_units([id("a"), id("b"), id("c")]).unwrap();
    assert!(!empty.matches(&workspace));
    let units = workspace.topology_stamp();
    assert!(workspace.add_units([id("a")]).is_err());
    workspace.retain_units(|_| true);
    assert!(units.matches(&workspace));
    workspace.add_link(link("one", "a", "b")).unwrap();
    assert!(!units.matches(&workspace));
    let connected = workspace.topology_stamp();
    assert!(workspace.connect(link("cycle", "b", "a")).is_err());
    assert!(workspace.add_link(link("one", "a", "b")).is_err());
    assert!(!workspace.remove_link(&id("missing")));
    assert!(connected.matches(&workspace));
    workspace.connect(link("replacement", "c", "b")).unwrap();
    assert!(!connected.matches(&workspace));
    let replaced = workspace.topology_stamp();
    assert!(workspace.remove_link(&id("replacement")));
    assert!(!replaced.matches(&workspace));
    let removed = workspace.topology_stamp();
    workspace.retain_units(|unit| *unit != id("c"));
    assert!(!removed.matches(&workspace));
    let retained = workspace.topology_stamp();
    workspace.clear();
    assert!(!retained.matches(&workspace));
}

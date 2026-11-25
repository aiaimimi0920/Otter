//! 身份批次原子性、主干删除级联规则和资源上限；不创建窗口或读写用户数据。
use super::*;
#[test]
fn instance_identity_survives_edits_and_empty_host_replacement_but_not_restore() {
    let mut workspace = Workspace::with_initial(id("a"));
    let stamp = workspace.identity_stamp();
    let topology = workspace.topology_stamp();
    workspace.add_units([id("b")]).unwrap();
    assert!(stamp.matches(&workspace));
    assert!(!topology.matches(&workspace));
    let replacement = workspace.replacement_unit(&id("a"), id("empty")).unwrap();
    assert!(stamp.matches(&replacement));
    assert_eq!(replacement.units(), &[id("b"), id("empty")]);
    assert_eq!(workspace.units(), &[id("a"), id("b")]);
    assert!(!stamp.matches(&Workspace::with_initial(id("empty"))));
    let first = Workspace::default();
    assert!(!first.identity_stamp().matches(&Workspace::default()));
}
fn id(value: &str) -> UnitId {
    UnitId::parse(value).unwrap()
}
fn link(value: &str, from: &str, to: &str) -> Link {
    Link {
        id: id(value),
        from_unit: id(from),
        from_port: "output".into(),
        to_unit: id(to),
        to_port: "image".into(),
    }
}
#[test]
fn batch_identity_conflicts_and_capacity_do_not_partially_register() {
    let mut model = Workspace::with_initial(id("a"));
    assert_eq!(model.add_units([id("b"), id("a")]), Err(Error::DuplicateId));
    assert_eq!(model.units(), &[id("a")]);
    assert_eq!(model.add_units([id("b"), id("b")]), Err(Error::DuplicateId));
    let many = (0..MAX_UNITS).map(|index| id(&format!("u{index}")));
    assert_eq!(model.add_units(many), Err(Error::Capacity));
    assert_eq!(model.units(), &[id("a")]);
}
#[test]
fn link_endpoints_ports_and_identity_are_checked_without_mutation() {
    let mut model = Workspace::default();
    model.add_units([id("a"), id("b")]).unwrap();
    assert_eq!(
        model.add_link(link("missing", "a", "absent")),
        Err(Error::MissingUnit)
    );
    let mut invalid = link("bad", "a", "b");
    invalid.to_port = "x\0y".into();
    assert_eq!(model.add_link(invalid), Err(Error::InvalidPort));
    assert!(model.links().is_empty());
    model.add_link(link("one", "a", "b")).unwrap();
    assert_eq!(
        model.add_link(link("one", "b", "a")),
        Err(Error::DuplicateId)
    );
    assert_eq!(model.links(), &[link("one", "a", "b")]);
}
#[test]
fn removing_units_cascades_both_directions_without_deleting_unrelated_edges() {
    let mut model = Workspace::default();
    model.add_units([id("a"), id("b"), id("c")]).unwrap();
    for edge in [
        link("ab", "a", "b"),
        link("ba", "b", "a"),
        link("bc", "b", "c"),
    ] {
        model.add_link(edge).unwrap();
    }
    model.retain_units(|unit| *unit != id("a"));
    assert_eq!(model.links(), &[link("bc", "b", "c")]);
    assert!(model.remove_link(&id("bc")));
    assert!(!model.remove_link(&id("bc")));
    model.retain_units(|_| false);
    assert!(model.units().is_empty());
    assert!(model.links().is_empty());
}
#[test]
fn link_budget_is_bounded_and_released_after_removal() {
    let mut model = Workspace::with_initial(id("a"));
    for index in 0..MAX_LINKS {
        model
            .add_link(link(&format!("l{index}"), "a", "a"))
            .unwrap();
    }
    assert_eq!(
        model.add_link(link("extra", "a", "a")),
        Err(Error::Capacity)
    );
    assert!(model.remove_link(&id("l0")));
    model.add_link(link("extra", "a", "a")).unwrap();
    assert_eq!(model.links().len(), MAX_LINKS);
}

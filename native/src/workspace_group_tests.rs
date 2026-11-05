//! 对照主干删除分组/关闭分组区别；纯模型测试不改变实际窗口可见性。
use super::*;
fn id(value: &str) -> UnitId {
    UnitId::parse(value).unwrap()
}
fn group(value: &str) -> Group {
    Group {
        id: id(value),
        name: value.into(),
        hidden: false,
        locked: false,
    }
}
fn model() -> Workspace {
    let mut workspace = Workspace::default();
    workspace.add_units([id("a"), id("b"), id("c")]).unwrap();
    workspace.upsert_group(group("one")).unwrap();
    workspace.upsert_group(group("two")).unwrap();
    workspace
}
#[test]
fn assignment_is_atomic_single_group_and_deduplicated() {
    let mut workspace = model();
    workspace
        .assign_group(&[id("a"), id("a"), id("b")], Some(&id("one")))
        .unwrap();
    assert_eq!(workspace.memberships.len(), 2);
    let before = workspace.memberships.clone();
    assert_eq!(
        workspace.assign_group(&[id("a"), id("missing")], Some(&id("two"))),
        Err(Error::MissingUnit)
    );
    assert_eq!(
        workspace.assign_group(&[id("a")], Some(&id("missing"))),
        Err(Error::MissingGroup)
    );
    assert_eq!(workspace.memberships, before);
    workspace
        .assign_group(&[id("a")], Some(&id("two")))
        .unwrap();
    assert_eq!(workspace.group_of(&id("a")).unwrap().id, id("two"));
    assert_eq!(workspace.group_of(&id("b")).unwrap().id, id("one"));
    workspace.assign_group(&[id("a")], None).unwrap();
    assert!(workspace.group_of(&id("a")).is_none());
}
#[test]
fn removing_group_keeps_units_while_closing_group_removes_members_and_edges() {
    let mut workspace = model();
    workspace
        .assign_group(&[id("a"), id("b")], Some(&id("one")))
        .unwrap();
    workspace
        .add_link(Link {
            id: id("link"),
            from_unit: id("a"),
            from_port: "output".into(),
            to_unit: id("c"),
            to_port: "image".into(),
        })
        .unwrap();
    assert!(workspace.remove_group(&id("one")));
    assert_eq!(workspace.units().len(), 3);
    assert_eq!(workspace.links().len(), 1);
    assert!(workspace.group_of(&id("a")).is_none());
    workspace
        .assign_group(&[id("b"), id("a")], Some(&id("two")))
        .unwrap();
    assert_eq!(
        workspace.close_group(&id("two")).unwrap(),
        vec![id("a"), id("b")]
    );
    assert_eq!(workspace.units(), &[id("c")]);
    assert!(workspace.links().is_empty());
    assert!(workspace.memberships.is_empty());
}
#[test]
fn flags_update_without_member_loss_and_closed_units_release_membership() {
    let mut workspace = model();
    workspace
        .assign_group(&[id("a")], Some(&id("one")))
        .unwrap();
    let mut updated = group("one");
    updated.name = "中文组".into();
    updated.hidden = true;
    updated.locked = true;
    workspace.upsert_group(updated.clone()).unwrap();
    assert_eq!(workspace.group_of(&id("a")), Some(&updated));
    let mut invalid = updated.clone();
    invalid.name = "x\0y".into();
    assert_eq!(workspace.upsert_group(invalid), Err(Error::InvalidName));
    assert_eq!(workspace.group_of(&id("a")), Some(&updated));
    workspace.retain_units(|unit| *unit != id("a"));
    assert!(workspace.memberships.is_empty());
    assert_eq!(workspace.groups().len(), 2);
    workspace.clear();
    assert!(workspace.groups().is_empty());
}

#[test]
fn group_capacity_name_bounds_and_missing_close_do_not_mutate_model() {
    let mut workspace = Workspace::default();
    for index in 0..MAX_UNITS {
        workspace.upsert_group(group(&format!("g{index}"))).unwrap();
    }
    assert_eq!(workspace.upsert_group(group("extra")), Err(Error::Capacity));
    assert_eq!(
        workspace.close_group(&id("absent")),
        Err(Error::MissingGroup)
    );
    for name in [String::new(), "a".repeat(257)] {
        let mut invalid = group("g0");
        invalid.name = name;
        assert_eq!(workspace.upsert_group(invalid), Err(Error::InvalidName));
    }
    assert_eq!(workspace.groups().len(), MAX_UNITS);
    assert!(workspace.close_group(&id("g0")).unwrap().is_empty());
    workspace.upsert_group(group("extra")).unwrap();
}

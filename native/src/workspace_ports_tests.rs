//! 外观模型与只读恢复边界；不访问真实profile或执行节点。
use super::*;
fn id(name: &str) -> UnitId {
    UnitId::parse(name).unwrap()
}
#[test]
fn visibility_roundtrip_chinese_reset_and_clone_are_independent() {
    let empty = Visibility::default();
    let hidden = empty.edited("中文🙂", Some(false)).unwrap();
    let shown = hidden.edited("中文🙂", Some(true)).unwrap();
    assert_eq!(empty.get("中文🙂"), None);
    assert_eq!(hidden.get("中文🙂"), Some(false));
    assert_eq!(
        Visibility::read(&serde_json::to_string(&shown).unwrap()).unwrap(),
        shown
    );
    assert!(shown.edited("中文🙂", None).unwrap().is_empty());
}
#[test]
fn malformed_duplicate_nonboolean_null_and_invalid_names_are_rejected() {
    for raw in [
        "null",
        "[]",
        "true",
        r#"{"a":null}"#,
        r#"{"a":0}"#,
        r#"{"a":true,"a":false}"#,
        r#"{"":true}"#,
        r#"{"a\n":true}"#,
    ] {
        assert!(Visibility::read(raw).is_err(), "{raw}");
    }
    for name in ["".into(), "x".repeat(257), "a\0b".into()] {
        assert!(Visibility::default().edited(&name, Some(true)).is_err());
    }
}
#[test]
fn count_and_byte_budgets_reject_without_changing_the_candidate() {
    let raw = serde_json::to_string(
        &(0..MAX_PORTS)
            .map(|i| (format!("p{i}"), true))
            .collect::<BTreeMap<_, _>>(),
    )
    .unwrap();
    let value = Visibility::read(&raw).unwrap();
    assert_eq!(value.edited("extra", Some(true)), Err(Error::Capacity));
    assert_eq!(value.get("extra"), None);
    assert!(Visibility::read(&format!("{}{{}}", " ".repeat(MAX_BYTES))).is_err());
    let large: BTreeMap<_, _> = (0..200)
        .map(|i| (format!("{i}{}", "x".repeat(200)), true))
        .collect();
    let raw = serde_json::to_string(&large).unwrap();
    assert!(Visibility::read(&raw).is_err());
    assert!(serde_json::from_str::<Visibility>(&raw).is_err());
}
#[test]
fn appearance_changes_do_not_retire_execution_identity_or_topology_or_links() {
    let unit = id("unit");
    let mut workspace = Workspace::with_initial(unit.clone());
    workspace.add_units([id("target")]).unwrap();
    workspace
        .add_link(crate::workspace::Link {
            id: id("link"),
            from_unit: unit.clone(),
            from_port: "same".into(),
            to_unit: id("target"),
            to_port: "same".into(),
        })
        .unwrap();
    let links = workspace.links().to_vec();
    let identity = workspace.identity_stamp();
    let topology = workspace.topology_stamp();
    let appearance = workspace.appearance_stamp();
    assert!(!workspace
        .set_port_visibility(&unit, Visibility::default())
        .unwrap());
    assert!(appearance.matches(&workspace));
    let candidate = workspace.clone();
    workspace
        .set_port_visibility(
            &unit,
            Visibility::default().edited("same", Some(false)).unwrap(),
        )
        .unwrap();
    assert!(!appearance.matches(&workspace));
    assert!(appearance.matches(&candidate));
    assert!(identity.matches(&workspace) && topology.matches(&workspace));
    assert_eq!(workspace.links(), links);
    let appearance = workspace.appearance_stamp();
    assert!(!workspace
        .set_port_visibility(&unit, workspace.port_visibility(&unit))
        .unwrap());
    assert!(appearance.matches(&workspace));
}
#[test]
fn deleted_units_and_clear_release_overrides_and_missing_unit_cannot_edit() {
    let unit = id("unit");
    let mut workspace = Workspace::with_initial(unit.clone());
    let value = Visibility::default().edited("output", Some(false)).unwrap();
    assert_eq!(
        workspace.set_port_visibility(&id("missing"), value.clone()),
        Err(Error::MissingUnit)
    );
    workspace.set_port_visibility(&unit, value.clone()).unwrap();
    workspace.retain_units(|_| false);
    assert!(workspace.visibility_records().is_empty());
    workspace.add_units([unit.clone()]).unwrap();
    assert!(workspace.port_visibility(&unit).is_empty());
    workspace.set_port_visibility(&unit, value).unwrap();
    workspace.clear();
    assert!(workspace.visibility_records().is_empty());
}
#[test]
fn session_old_absence_and_explicit_overrides_restore_but_invalid_candidate_fails() {
    for field in [
        "".to_owned(),
        r#", "portVisibility":{"中文":false,"output":true}"#.into(),
    ] {
        let raw = format!(
            r#"{{"future":{{"keep":1}},"stickers":[{{"id":"unit","x":0,"y":0,"w":1,"h":1{field}}}],"links":[]}}"#
        );
        let doc = crate::session_document::decode(raw.as_bytes()).unwrap();
        assert_eq!(doc.original_json(), raw.as_bytes());
        assert_eq!(
            doc.workspace.port_visibility(&id("unit")).get("中文"),
            (!field.is_empty()).then_some(false)
        );
    }
    for invalid in ["null", "[]", r#"{"a":true,"a":false}"#, r#"{"a":"false"}"#] {
        let raw = format!(
            r#"{{"stickers":[{{"id":"unit","x":0,"y":0,"w":1,"h":1,"portVisibility":{invalid}}}],"links":[]}}"#
        );
        assert!(crate::session_document::decode(raw.as_bytes()).is_err());
    }
}

//! 合成旧会话字段的拓扑往返；不接触用户会话文件。
use super::*;
fn fixture() -> Topology {
    Topology {
        stickers: serde_json::from_value(serde_json::json!([
            {"id":"钉图-a","x":-200,"src":"untouched","groupId":"g"},
            {"id":"art-b","groupId":null}, {"id":"sticker-c"}
        ])).unwrap(),
        links: serde_json::from_value(serde_json::json!([
            {"id":"edge-1","fromUnitId":"钉图-a","fromPortId":"output","toUnitId":"art-b","toPortId":"image"},
            {"id":"edge-2","fromUnitId":"art-b","fromPortId":"output","toUnitId":"钉图-a","toPortId":"image"}
        ])).unwrap(),
        groups: serde_json::from_value(serde_json::json!([{"id":"g","name":"参考组","hidden":true,"locked":true}])).unwrap(),
    }
}
#[test]
fn legacy_fields_roundtrip_without_renaming_ids_or_rewriting_edges() {
    let original = fixture();
    let restored = original.restore().unwrap();
    assert_eq!(Topology::capture(&restored), original);
    assert_eq!(
        restored.links().len(),
        2,
        "import must preserve historical cycles"
    );
    let link = serde_json::to_value(&original.links[0]).unwrap();
    assert_eq!(link["fromUnitId"], "钉图-a");
    assert!(link.get("from_unit_id").is_none());
    let default: SessionGroup = serde_json::from_str(r#"{"id":"g","name":"Group"}"#).unwrap();
    assert!(!default.hidden && !default.locked);
}
#[test]
fn malformed_restore_does_not_change_existing_workspace() {
    let current = fixture().restore().unwrap();
    let stamp = current.topology_stamp();
    let before = Topology::capture(&current);
    for case in 0..5 {
        let mut invalid = fixture();
        match case {
            0 => invalid.stickers.push(invalid.stickers[0].clone()),
            1 => invalid.groups.push(invalid.groups[0].clone()),
            2 => invalid.stickers[0].group_id = Some("missing".into()),
            3 => invalid.links[0].from_unit_id = "missing".into(),
            _ => invalid.links[0].to_port_id = "bad\nport".into(),
        }
        assert!(invalid.restore().is_err());
        assert_eq!(Topology::capture(&current), before);
        assert!(stamp.matches(&current));
    }
}
#[test]
fn restored_workspace_gets_fresh_runtime_topology_identity() {
    let old = fixture().restore().unwrap();
    let new = Topology::capture(&old).restore().unwrap();
    assert!(!old.topology_stamp().matches(&new));
    let mut too_many = fixture();
    too_many
        .links
        .resize(MAX_LINKS + 1, too_many.links[0].clone());
    assert!(matches!(
        too_many.restore(),
        Err(Error::Model(ModelError::Capacity))
    ));
}

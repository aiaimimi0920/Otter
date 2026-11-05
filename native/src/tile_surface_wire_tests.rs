//! 独立JSON oracle覆盖Wall/Surface身份、版本回退和资源/结构预算，不从解析器生成预期。
use super::*;
use serde_json::json;
pub(crate) fn fixture() -> (Binding, Value) {
    let binding = Binding {
        endpoint_id: "tile".into(),
        lease_id: "lease".into(),
        revision: 2,
    };
    let v = json!({"protocolVersion":"loom.wall.v1", "view":{"binding":{"endpointId":"tile","leaseId":"lease","revision":2},"instanceId":"art","attachmentId":"view"},
        "width":64,"height":64,"generation":1,"sequence":0,"snapshot":{"protocolVersion":"loom.surface.v1","instanceId":"art","attachmentId":"view","artId":"fixture","artVersion":"1.0.0","revision":1,"runtime":"declarative","scene":{"id":"root","type":"view","style":{"background":"#ff0000"}},"resources":[],"resourceLeases":[]},
        "preview":null,"result":null,"confirmations":[],"pending":[],"failure":null});
    (binding, v)
}
fn parse(v: &Value, b: &Binding, old: Option<&State>) -> Result<State> {
    State::parse(&serde_json::to_vec(v).unwrap(), b, "art", old)
}
#[test]
fn new_instance_generation_zero_is_valid_but_cannot_replace_a_later_generation() {
    let (b, mut v) = fixture();
    v["generation"] = 0.into();
    let first = parse(&v, &b, None).unwrap();
    assert_eq!(first.generation, 0);
    assert_eq!(parse(&v, &b, Some(&first)).unwrap().generation, 0);
    v["generation"] = 1.into();
    let next = parse(&v, &b, Some(&first)).unwrap();
    v["generation"] = 0.into();
    assert!(parse(&v, &b, Some(&next)).is_err());
    v["generation"] = (wall::MAX_SAFE + 1).into();
    assert!(parse(&v, &b, None).is_err());
}
#[test]
fn unchanged_snapshot_keeps_exact_owner_and_rejects_null_first_or_same_revision_mutation() {
    let (b, mut v) = fixture();
    let first = parse(&v, &b, None).unwrap();
    let next = parse(&v, &b, Some(&first)).unwrap();
    assert!(Arc::ptr_eq(&first.snapshot, &next.snapshot));
    v["snapshot"] = Value::Null;
    assert!(parse(&v, &b, None).is_err());
    assert!(Arc::ptr_eq(
        &first.snapshot,
        &parse(&v, &b, Some(&first)).unwrap().snapshot
    ));
    let (_, mut v) = fixture();
    v["snapshot"]["scene"]["style"]["background"] = "#00ff00".into();
    assert!(parse(&v, &b, Some(&first)).is_err());
    v["snapshot"]["revision"] = 2.into();
    let second = parse(&v, &b, Some(&first)).unwrap();
    let (_, original) = fixture();
    assert!(parse(&original, &b, Some(&second)).is_err());
}
#[test]
fn identity_runtime_transferable_lease_and_inline_image_are_rejected() {
    let (b, base) = fixture();
    for path in [
        "/protocolVersion",
        "/view/binding/endpointId",
        "/view/binding/leaseId",
        "/view/instanceId",
        "/snapshot/instanceId",
        "/snapshot/attachmentId",
        "/snapshot/runtime",
    ] {
        let mut v = base.clone();
        *v.pointer_mut(path).unwrap() = "wrong".into();
        assert!(parse(&v, &b, None).is_err(), "{path}");
    }
    let mut v = base.clone();
    v["snapshot"]["resourceLeases"] = json!([{"leaseId":"private"}]);
    assert!(parse(&v, &b, None).is_err());
    for props in [
        json!({"src":"data:image/png;base64,AAAA"}),
        json!({"resourceId":format!("sha256:{}","a".repeat(64))}),
        json!({"src":"https://example.invalid/image"}),
    ] {
        let mut v = base.clone();
        v["snapshot"]["scene"] = json!({"id":"image","type":"image","props":props});
        assert!(parse(&v, &b, None).is_err());
    }
}
#[test]
fn node_depth_total_and_json_string_budgets_are_independent_of_general_scene_limits() {
    let (b, base) = fixture();
    let mut v = base.clone();
    v["snapshot"]["scene"]["children"] = Value::Array(
        (0..512)
            .map(|n| json!({"id":format!("n{n}"),"type":"spacer"}))
            .collect(),
    );
    assert!(parse(&v, &b, None).is_err());
    v["snapshot"]["scene"]["children"]
        .as_array_mut()
        .unwrap()
        .pop();
    assert_eq!(parse(&v, &b, None).unwrap().snapshot.nodes, 512);
    let mut node = json!({"id":"leaf","type":"text"});
    for n in 0..17 {
        node = json!({"id":format!("p{n}"),"type":"view","children":[node]});
    }
    v["snapshot"]["scene"] = node;
    assert!(parse(&v, &b, None).is_err());
    let mut v = base;
    v["unused"] = "x".repeat(2 * 1024 * 1024).into();
    assert!(parse(&v, &b, None).is_err());
}
#[test]
fn preview_formal_and_instance_generation_never_roll_back_or_replace_scene() {
    let (b, mut v) = fixture();
    v["preview"] = json!({"protocolVersion":"loom.surface.v1","instanceId":"art","requestId":"preview","generation":1,"previewRevision":5});
    v["result"] = json!({"protocolVersion":"loom.surface.v1","instanceId":"art","requestId":"result","generation":1,"resultRevision":3});
    let first = parse(&v, &b, None).unwrap();
    v["snapshot"] = Value::Null;
    assert!(Arc::ptr_eq(
        &first.snapshot,
        &parse(&v, &b, Some(&first)).unwrap().snapshot
    ));
    for (path, value) in [
        ("/preview/previewRevision", 4),
        ("/result/resultRevision", 2),
        ("/generation", 0),
    ] {
        let mut next = v.clone();
        *next.pointer_mut(path).unwrap() = value.into();
        assert!(parse(&next, &b, Some(&first)).is_err(), "{path}");
    }
}

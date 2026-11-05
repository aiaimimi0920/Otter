//! 会话准备阶段的结果恢复集成测试；无网络访问，不发布活动窗口。
use super::*;

#[test]
fn saved_json_outputs_roundtrip_through_real_session_candidate() {
    let cancel = AtomicBool::new(false);
    for outputs in [
        serde_json::json!({}),
        serde_json::json!({
            "value": false, "resource": {"uri":"opaque:test","extra":3},
            "stream": {"streamId":"stream:test"}
        }),
    ] {
        let bytes = serde_json::to_vec(&serde_json::json!({"stickers":[{
            "id":"art", "type":"art", "x":0,"y":0,"w":2,"h":2,
            "outputs": outputs
        }],"links":[]}))
        .unwrap();
        let candidate = prepare(bytes, None, &cancel).unwrap();
        let results = candidate.prepare_results(&cancel).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(
            serde_json::Value::Object(results[0].save_value_outputs().unwrap()),
            outputs
        );
        assert!(results[0].active().is_none());
        assert!(results[0].surface_result_revision().is_none());
        assert_eq!(
            results[0].status(),
            crate::desktop::loom_result_state::Status::Idle
        );
    }
}

#[test]
fn invalid_later_node_rejects_entire_result_batch_without_mutating_candidate() {
    let bytes = br#"{"stickers":[
        {"id":"first","type":"art","x":0,"y":0,"w":2,"h":2,"outputs":{"ok":1}},
        {"id":"second","type":"art","x":0,"y":0,"w":2,"h":2,"outputs":{"same":1,"same":2}}
    ],"links":[]}"#
        .to_vec();
    let cancel = AtomicBool::new(false);
    let candidate = prepare(bytes.clone(), None, &cancel).unwrap();
    assert!(candidate.prepare_results(&cancel).is_err());
    assert_eq!(candidate.original_json(), bytes);
    assert!(candidate.prepare_results(&AtomicBool::new(true)).is_err());
}

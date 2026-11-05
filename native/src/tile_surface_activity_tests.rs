//! 独立不可信控制JSON oracle：仅本view身份、执行状态集合、容量与脱敏失败。
use super::*;
use crate::tile_surface_actions::tests::{ack_value, confirmation, value, Fixture};
use serde_json::json;
#[test]
fn pending_accepts_only_executing_statuses_and_unique_owned_requests() {
    let f = Fixture::new("art");
    for status in [
        "accepted",
        "queued",
        "running",
        "cancel_requested",
        "awaiting_confirmation",
        "succeeded",
        "failed",
        "cancelled",
        "unknown",
    ] {
        let mut v = value(&f.job.key, "art");
        v["pending"] =
            json!([{"ack":ack_value("event",status),"actionId":"run","cancelable":true}]);
        let ok = Activity::parse(&v, &f.state.view).is_ok();
        assert_eq!(
            ok,
            ["accepted", "queued", "running", "cancel_requested"].contains(&status),
            "{status}"
        );
    }
    let mut v = value(&f.job.key, "art");
    let p = json!({"ack":ack_value("event","running"),"actionId":"run","cancelable":true});
    v["pending"] = json!([p, p]);
    assert!(Activity::parse(&v, &f.state.view).is_err());
    v["pending"] = json!([{"ack":ack_value("event","running"),"actionId":"run","cancelable":true}]);
    v["pending"][0]["ack"]["instanceId"] = "other".into();
    assert!(Activity::parse(&v, &f.state.view).is_err());
}
#[test]
fn confirmation_identity_payload_bounds_and_failure_code_are_not_trusted() {
    let f = Fixture::new("art");
    let mut v = value(&f.job.key, "art");
    let good = confirmation("event", &f.job.key.endpoint);
    v["confirmations"] = json!([good]);
    assert!(Activity::parse(&v, &f.state.view).is_ok());
    for key in [
        "protocolVersion",
        "instanceId",
        "attachmentId",
        "hookNodeId",
        "risk",
    ] {
        let mut bad = v.clone();
        bad["confirmations"][0][key] = "wrong".into();
        assert!(Activity::parse(&bad, &f.state.view).is_err(), "{key}");
    }
    let mut bad = v.clone();
    bad["confirmations"][0]["payload"] = json!({"large":"x".repeat(64*1024)});
    assert!(Activity::parse(&bad, &f.state.view).is_err());
    let mut bad = v.clone();
    bad["confirmations"] = json!(vec![good; 65]);
    assert!(Activity::parse(&bad, &f.state.view).is_err());
    v["failure"] = json!({"requestId":"request","code":"wall_surface_action_failed"});
    assert_eq!(
        Activity::parse(&v, &f.state.view)
            .unwrap()
            .failure
            .as_deref(),
        Some("request")
    );
    v["failure"]["code"] = "raw_runtime_error".into();
    assert!(Activity::parse(&v, &f.state.view).is_err());
}

//! 对照主干 action_dispatch 的控制协议，验证全局/来源身份和边界，而非前端 action 包。
use super::*;
use crate::session_document::workflow_snapshot::{self, Content};
use serde_json::{json, Value};

fn fixture() -> (Vec<u8>, workflow_snapshot::Snapshot) {
    let saved = serde_json::to_vec(&json!({"documentRevision":7,"stickers":[{
        "id":"local","x":1,"y":2,"w":3,"h":4,"originWorkflowId":"remote","originNodeId":"origin"}],"links":[]})).unwrap();
    let snapshot = workflow_snapshot::prepare(&saved, 7, &AtomicBool::new(false), |_, _| {
        Ok(Content {
            parameters: serde_json::Map::new(),
            image: serde_json::Map::new(),
        })
    })
    .unwrap();
    (saved, snapshot)
}
#[test]
fn global_and_component_requests_use_control_method_and_camel_case_params() {
    let (saved, snapshot) = fixture();
    let cancel = AtomicBool::new(false);
    let global: Value = serde_json::from_slice(
        &snapshot
            .encode_request("workflow-sync:one", 4096, &cancel)
            .unwrap(),
    )
    .unwrap();
    assert_eq!(global["method"], "loom.hook.workflow.sync");
    assert_eq!(global["params"]["requestId"], "workflow-sync:one");
    assert_eq!(global["params"]["workflowId"], "hook-live");
    assert_eq!(global["params"]["snapshot"]["nodes"][0]["id"], "local");
    assert!(global.get("action").is_none());
    assert!(global["params"].get("workflow_id").is_none());
    let components = workflow_snapshot::components::prepare(&snapshot, &saved, &cancel).unwrap();
    let request: Value = serde_json::from_slice(
        &components[0]
            .encode_request("workflow-sync:two", 4096, &cancel)
            .unwrap(),
    )
    .unwrap();
    assert_eq!(request["params"]["workflowId"], "remote");
    assert_eq!(request["params"]["snapshot"]["nodes"][0]["id"], "origin");
    assert_eq!(request["params"]["snapshot"]["documentRevision"], 7);
    assert_eq!(snapshot.value()["nodes"][0]["id"], "local");
}
#[test]
fn request_ids_are_escaped_and_entire_envelope_is_bounded() {
    let (saved, snapshot) = fixture();
    let cancel = AtomicBool::new(false);
    let id = "request:\"\\中文";
    let encoded = snapshot.encode_request(id, 4096, &cancel).unwrap();
    assert_eq!(
        serde_json::from_slice::<Value>(&encoded).unwrap()["params"]["requestId"],
        id
    );
    assert!(snapshot.encode_request(id, encoded.len(), &cancel).is_ok());
    assert_eq!(
        snapshot.encode_request(id, encoded.len() - 1, &cancel),
        Err(Error::Budget)
    );
    let components = workflow_snapshot::components::prepare(&snapshot, &saved, &cancel).unwrap();
    for id in ["".to_string(), "x".repeat(257), "bad\nidentity".into()] {
        assert_eq!(
            snapshot.encode_request(&id, 4096, &cancel),
            Err(Error::RequestIdentity)
        );
        assert_eq!(
            components[0].encode_request(&id, 4096, &cancel),
            Err(Error::RequestIdentity)
        );
    }
    assert_eq!(
        snapshot.encode_request("valid", 4096, &AtomicBool::new(true)),
        Err(Error::Cancelled)
    );
    assert_eq!(
        components[0].encode_request("valid", 4096, &AtomicBool::new(true)),
        Err(Error::Cancelled)
    );
}

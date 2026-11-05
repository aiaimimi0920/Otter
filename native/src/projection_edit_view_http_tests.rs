//! 实际pinned TLS发送独立数值oracle核过的局部编辑；不以模型或显式ack冒充Application保存。
use super::*;
use crate::projection_edit::{
    fixture::*,
    journal::Store,
    view::{reconcile, Size, View},
    Operation, Request, Route,
};
use serde_json::{json, Value};
use std::sync::atomic::AtomicBool;

#[test]
fn nonuniform_local_edit_uses_real_tls_original_journal_and_retains_changes_after_reply() {
    let (listener, client, config) = fixture();
    let authorized = session(client.origin(), u64::MAX);
    let directory = Directory::new();
    let store: Store = directory.store();
    let cancel = AtomicBool::new(false);
    let mut returned = document_value();
    returned["revision"] = 2.into();
    returned["receiptCount"] = 1.into();
    returned["objects"]["shape-1"]["revision"] = 2.into();
    returned["objects"]["shape-1"]["value"]["x"] = 7.into();
    let response = serde_json::to_vec(&returned).unwrap();
    let peer = std::thread::spawn(move || {
        let expected = [
            json!({"operation":"initialize","projectionId":PROJECTION,"sessionId":SESSION,
            "expectedDigest":DIGEST,"objects":{"shape-1":{"id":"shape-1","type":"rect","zIndex":0,
                "style":{"color":"#123456","width":2},"x":3,"y":4,"w":10,"h":20}}}),
            json!({"operation":"apply","projectionId":PROJECTION,"sessionId":SESSION,"opId":"op-view-tls",
            "baseRevision":1,"modeRevision":1,"changes":[{"objectId":"shape-1","value":{
                "id":"shape-1","type":"rect","zIndex":0,"style":{"color":"#123456","width":2},"x":7,"y":4,"w":10,"h":20}}]}),
        ];
        for (index, expected) in expected.into_iter().enumerate() {
            let mut stream = StreamOwned::new(
                ServerConnection::new(config.clone()).unwrap(),
                accept(&listener),
            );
            let (head, body) = request(&mut stream);
            assert!(head.starts_with("POST /v1/projections/edit HTTP/1.1\r\n"));
            assert!(head
                .to_ascii_lowercase()
                .contains("authorization: device fixture-secret\r\n"));
            // JSON数值表示可以为7或7.0；逐字段独立比较，不调用被测几何层。
            assert_eq!(body["operation"], expected["operation"]);
            if index == 0 {
                assert_eq!(body, expected);
            } else {
                assert_eq!(body["opId"], "op-view-tls");
                assert_eq!(body["baseRevision"], 1);
                assert_eq!(body["modeRevision"], 1);
                assert_eq!(body["projectionId"], PROJECTION);
                assert_eq!(body["sessionId"], SESSION);
                assert_eq!(body["changes"].as_array().unwrap().len(), 1);
                assert_eq!(body["changes"][0]["objectId"], "shape-1");
                let actual = &body["changes"][0]["value"];
                let expected = &expected["changes"][0]["value"];
                assert_eq!(
                    actual.as_object().unwrap().len(),
                    expected.as_object().unwrap().len()
                );
                for key in ["x", "y", "w", "h", "zIndex"] {
                    assert_eq!(actual[key].as_f64(), expected[key].as_f64());
                }
                assert_eq!(actual["id"], "shape-1");
                assert_eq!(actual["type"], "rect");
                assert_eq!(actual["style"]["width"].as_f64(), Some(2.0));
                assert_eq!(actual["style"]["color"], "#123456");
                assert_eq!(actual["style"].as_object().unwrap().len(), 2);
                assert!(actual.get("showArrowHead").is_none());
            }
            if index == 0 {
                reply(
                    &mut stream,
                    200,
                    &serde_json::to_vec(&document_value()).unwrap(),
                );
            } else {
                reply(&mut stream, 200, &response);
            }
        }
    });
    let ticket = store
        .begin(
            context(client.origin()),
            Route::Shared,
            initialize(),
            None,
            view(),
            &cancel,
        )
        .unwrap();
    let remote = Control::new(&client, &authorized, &Cancel::new())
        .projection_edit(&ticket)
        .unwrap();
    ticket
        .commit_after_checkpoint(remote.clone(), &cancel)
        .unwrap();
    let observed = View::from_document(&remote, Size::new(32.0, 16.0).unwrap()).unwrap();
    let resized = observed.resized(Size::new(64.0, 16.0).unwrap());
    let mut local = resized.local_objects().unwrap();
    local.get_mut("shape-1").unwrap()["x"] = 14.into();
    let edited = resized.edited(local).unwrap();
    let request = Request::prepare(Operation::Apply {
        projection_id: PROJECTION.into(),
        session_id: SESSION.into(),
        op_id: "op-view-tls".into(),
        base_revision: 1,
        mode_revision: 1,
        changes: observed.changes_to(&edited).unwrap(),
    })
    .unwrap();
    let ticket = store
        .begin(
            context(client.origin()),
            Route::Shared,
            request,
            Some(remote.clone()),
            edited.basis_objects().clone(),
            &cancel,
        )
        .unwrap();
    let before = std::fs::read(directory.path()).unwrap();
    let result = Control::new(&client, &authorized, &Cancel::new())
        .projection_edit(&ticket)
        .unwrap();
    peer.join().unwrap();
    assert_eq!(store.pending(&cancel).unwrap().len(), 1);
    assert_eq!(std::fs::read(directory.path()).unwrap(), before);
    let mut later = edited.local_objects().unwrap();
    later.get_mut("shape-1").unwrap()["x"] = 18.into();
    let later = edited.edited(later).unwrap();
    let merged = reconcile(&remote, &edited, &result, &later, None).unwrap();
    assert!(merged.conflict().is_none());
    assert_eq!(
        merged.authoritative().basis_objects()["shape-1"]["x"].as_f64(),
        Some(7.0)
    );
    assert_eq!(
        merged.visible().basis_objects()["shape-1"]["x"].as_f64(),
        Some(9.0)
    );
    assert_eq!(merged.changes().len(), 1);
    let request: Value = serde_json::from_slice(ticket.request().bytes()).unwrap();
    assert_eq!(request["changes"][0]["value"]["x"].as_f64(), Some(7.0));
}

//! 真实TLS配置与控制同actor串行；固定wire和下一控制命令序号独立核验。
use super::*;
use crate::device_session_client::tests::{accept, fixture_with_pem, reply, request, session};
use crate::live_relay_trigger_command::Command;
use rustls::{ServerConnection, StreamOwned};
use serde_json::json;
#[test]
fn release_precedes_trigger_and_confirmation_advances_same_actor_before_acquire() {
    let (listener, client, tls, _) = fixture_with_pem();
    let credential = session(client.origin(), u64::MAX);
    let response = json!({"session":{"protocolVersion":"loom.live.v1","sessionId":"live:queue",
        "sourceDeviceId":"source-1","sourceHookId":"hook-1","revision":2,"viewerDevices":["device-1"],
        "interactionCapabilities":["keyboard","cancel"],"observationCapabilities":[]},"epoch":1,"closed":false,
        "observations":[],"triggers":[],"requesterControl":{"deviceId":"device-1","epoch":1,"controlSequence":1,"inputSequence":0}});
    let command = Command::parse(r#"{"bindingId":"b:1","enabled":true,"target":{"surfaceInstanceId":"surface:1","surfaceAttachmentId":"attachment:1","surfaceNodeId":"finish","surfaceEvent":"click","surfaceAction":"finish"},"condition":{"conditionId":"c:1","revision":1,"observationId":"uia:edit","operator":"equals","operand":true,"stableForMs":0,"risingEdge":false,"rearm":false,"minimumConfidence":"exact"}}"#).unwrap();
    let expected = command.clone();
    let initial = response.clone();
    let peer = std::thread::spawn(move || {
        let mut snapshot = initial;
        for stage in 0..5 {
            let mut stream = StreamOwned::new(
                ServerConnection::new(tls.clone()).unwrap(),
                accept(&listener),
            );
            let (head, body) = request(&mut stream);
            if stage == 0 {
                assert!(head.contains("/control "));
                assert_eq!(body["action"], "release");
                assert_eq!(body["sequence"], 2);
                snapshot["requesterControl"]["controlSequence"] = json!(2);
            } else if stage == 2 {
                assert!(head.contains("/triggers "));
                assert_eq!(body["envelope"]["sequence"], 3);
                snapshot["requesterControl"]["controlSequence"] = json!(3);
                snapshot["triggers"] = json!([{"binding":{"bindingId":"b:1","enabled":true,"authorizedBy":"device-1","observationId":"uia:edit","conditionRevision":1},"condition":expected.condition,"target":expected.target,"armed":true,"lastMatch":false,"pendingOfflineMatch":false}]);
            } else if stage == 4 {
                assert!(head.contains("/control "));
                assert_eq!(body["sequence"], 4);
                snapshot["session"]["controllerDevice"] = json!("device-1");
            } else {
                assert!(head.starts_with("GET "));
            }
            reply(&mut stream, 200, &serde_json::to_vec(&snapshot).unwrap());
        }
    });
    let snapshot = crate::live_relay_viewer_identity::Snapshot::parse(
        &serde_json::to_vec(&response).unwrap(),
        "live:queue",
    )
    .unwrap();
    let identity = snapshot.identity().clone();
    let cursor = crate::live_relay_viewer_identity::ControlCursor::new(identity.clone(), 0);
    // 批次必须由随后移交Owner的同一个cursor准备，不能伪造另一owner的许可。
    let bootstrap = cursor.prepare(br#"{"protocolVersion":"loom.live.v1","next":1,"reset":false,"events":[{"protocolVersion":"loom.live.v1","sessionId":"live:queue","epoch":1,"sequence":1,"messageType":"session_state","payload":{"revision":2,"reason":"viewer_joined","visibility":"visible","viewers":["device-1"]}}]}"#).unwrap();
    let prepared = crate::live_relay_http::Prepared {
        business: crate::live_relay_viewer_business::Business::from_snapshot(
            &serde_json::to_vec(&response).unwrap(),
        )
        .unwrap(),
        snapshot,
        cursor,
        control_sequence: 1,
        input_sequence: 0,
    };
    let mut owner = Owner::new(prepared, "device-1".into()).unwrap();
    owner.apply(bootstrap, |_| Ok(())).unwrap();
    let queue = Arc::new(Queue::default());
    queue.configure(owner.business().interaction(), &owner.media());
    queue
        .triggers
        .configure(Some(("surface:1".into(), "attachment:1".into())));
    assert!(queue.triggers.submit(command));
    let mut sender = Sender::new(
        queue.clone(),
        RequesterCursor {
            control: 1,
            input: 0,
        },
    );
    let cancel = crate::device_request_cancel::Cancel::default();
    let control = Control::new(&client, &credential, &cancel);
    queue.request();
    queue.mark_possible();
    queue.acknowledge(true);
    sender.owned = true;
    queue.release();
    let target = RequestTarget::Surface(Target {
        session: "live:queue".into(),
        surface: "surface:1".into(),
        attachment: "attachment:1".into(),
        expected: None,
        selected: Some(identity.clone()),
    });
    assert!(!sender
        .drive_commands(&control, &target, &mut owner)
        .unwrap());
    assert_eq!(
        sender.cursor,
        RequesterCursor {
            control: 3,
            input: 0
        }
    );
    assert_eq!(owner.business().registrations().len(), 1);
    assert_eq!(owner.after(), 1);
    assert!(queue.request());
    sender.drive(&control, &target, &identity, 1).unwrap();
    assert_eq!(sender.cursor.control, 4);
    assert!(queue.status().enabled);
    peer.join().unwrap();
}

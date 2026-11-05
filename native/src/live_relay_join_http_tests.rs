//! 真实 HTTPS 串联发现、成员基线恢复、加入和历史起点；不启动媒体，不访问用户身份。
use super::*;
use crate::{live_relay_http::Join, live_relay_viewer_identity::Snapshot};

#[test]
fn attach_rejects_missing_business_snapshot_after_authenticated_membership_check() {
    let (listener, client, config) = fixture();
    let session = session(&client.origin, u64::MAX);
    let expected = Snapshot::parse(&snapshot(), "live:test/part").unwrap();
    let peer = std::thread::spawn(move || {
        let mut stream =
            StreamOwned::new(ServerConnection::new(config).unwrap(), accept(&listener));
        let (head, _) = request(&mut stream);
        assert!(head.starts_with("POST "));
        let mut value: serde_json::Value = serde_json::from_slice(&snapshot()).unwrap();
        value.as_object_mut().unwrap().remove("observations");
        reply(&mut stream, 200, &serde_json::to_vec(&value).unwrap());
    });
    let cancel = Cancel::new();
    let result = Control::new(&client, &session, &cancel).attach(
        expected.identity(),
        Attach {
            surface: "surface-1",
            attachment: "attachment-1",
            sequence: 1,
            require_existing_membership: false,
        },
    );
    assert!(matches!(result, Err(LiveError::Business(_))));
    peer.join().unwrap();
}

#[test]
fn prepare_uses_zero_for_new_member_and_authenticated_actor_cursor_for_renewal() {
    for member in [false, true] {
        let (listener, client, config) = fixture();
        let session = session(&client.origin, u64::MAX);
        let expected = Snapshot::parse(&snapshot(), "live:test/part").unwrap();
        let peer = std::thread::spawn(move || {
            let count = if member { 4 } else { 3 };
            for index in 0..count {
                let mut stream = StreamOwned::new(
                    ServerConnection::new(config.clone()).unwrap(),
                    accept(&listener),
                );
                let (head, body) = request(&mut stream);
                let mut value: serde_json::Value = serde_json::from_slice(&snapshot()).unwrap();
                if index == 0 {
                    assert!(head.starts_with("GET /v1/live/sessions HTTP/1.1"));
                    if !member {
                        value["session"]["viewerDevices"] = serde_json::json!([]);
                    }
                    reply(
                        &mut stream,
                        200,
                        &serde_json::to_vec(&serde_json::json!({
                        "protocolVersion":"loom.live.v1", "sessions":[value]}))
                        .unwrap(),
                    );
                } else if member && index == 1 {
                    assert!(head.starts_with("GET /v1/live/sessions/live:test%2Fpart HTTP/1.1"));
                    value["requesterControl"] = serde_json::json!({"deviceId":"device-1","epoch":1,
                        "controlSequence":8,"inputSequence":3});
                    reply(&mut stream, 200, &serde_json::to_vec(&value).unwrap());
                } else if index == count - 1 {
                    assert!(head.starts_with(
                        "GET /v1/live/sessions/live:test%2Fpart/events?after=0&timeoutMs=0 "
                    ));
                    reply(&mut stream, 200, &history(true));
                } else {
                    assert!(head.starts_with("POST /v1/live/sessions/live:test%2Fpart/viewers "));
                    assert_eq!(body["envelope"]["sequence"], if member { 9 } else { 1 });
                    if member {
                        assert_eq!(body["requireExistingMembership"], true);
                    } else {
                        assert!(body.get("requireExistingMembership").is_none());
                    }
                    value["session"]["observationCapabilities"] = serde_json::json!(["adapter"]);
                    value["observations"] = serde_json::json!([{"observationId":"o","sequence":9,
                        "state":"stable","source":"app_adapter","confidence":"high","observedAtMs":1,"value":42}]);
                    reply(&mut stream, 200, &serde_json::to_vec(&value).unwrap());
                }
            }
        });
        let cancel = Cancel::new();
        let prepared = Control::new(&client, &session, &cancel)
            .prepare_viewer(Join {
                session: "live:test/part",
                surface: "surface-1",
                attachment: "attachment-1",
                expected: member.then(|| expected.identity()),
            })
            .unwrap();
        assert_eq!(prepared.control_sequence, if member { 9 } else { 1 });
        assert_eq!(prepared.input_sequence, if member { 3 } else { 0 });
        assert_eq!(prepared.cursor.after(), 6);
        assert_eq!(prepared.snapshot.identity(), expected.identity());
        assert_eq!(
            prepared.business.observations().get("o").unwrap().sequence,
            9
        );
        assert_eq!(prepared.business.capabilities().values().len(), 1);
        peer.join().unwrap();
    }
}

#[test]
fn removed_renewal_member_and_wrong_actor_fail_before_any_attach_request() {
    for removed in [false, true] {
        let (listener, client, config) = fixture();
        let session = session(&client.origin, u64::MAX);
        let expected = Snapshot::parse(&snapshot(), "live:test/part").unwrap();
        let peer = std::thread::spawn(move || {
            let count = if removed { 1 } else { 2 };
            for index in 0..count {
                let mut stream = StreamOwned::new(
                    ServerConnection::new(config.clone()).unwrap(),
                    accept(&listener),
                );
                let (head, _) = request(&mut stream);
                assert!(head.starts_with("GET "));
                let mut value: serde_json::Value = serde_json::from_slice(&snapshot()).unwrap();
                if index == 0 {
                    if removed {
                        value["session"]["viewerDevices"] = serde_json::json!([]);
                    }
                    value =
                        serde_json::json!({"protocolVersion":"loom.live.v1", "sessions":[value]});
                } else {
                    value["requesterControl"] = serde_json::json!({"deviceId":"other-device", "epoch":1,
                        "controlSequence":999,"inputSequence":0});
                }
                reply(&mut stream, 200, &serde_json::to_vec(&value).unwrap());
            }
            listener
        });
        let cancel = Cancel::new();
        let result = Control::new(&client, &session, &cancel).prepare_viewer(Join {
            session: "live:test/part",
            surface: "surface-1",
            attachment: "attachment-1",
            expected: Some(expected.identity()),
        });
        if removed {
            assert!(matches!(result, Err(LiveError::Protocol(_))));
        } else {
            assert!(matches!(result, Err(LiveError::Cursor)));
        }
        let listener = peer.join().unwrap();
        assert!(matches!(listener.accept(), Err(e) if e.kind() == io::ErrorKind::WouldBlock));
    }
}

#[test]
fn resume_returns_next_sequence_only_for_valid_success_without_guessing_from_conflict() {
    use crate::live_relay_http::RequesterCursor;
    for scenario in 0..3 {
        let (listener, client, config) = fixture();
        let session = session(&client.origin, u64::MAX);
        let expected = Snapshot::parse(&snapshot(), "live:test/part").unwrap();
        let peer = std::thread::spawn(move || {
            let mut stream =
                StreamOwned::new(ServerConnection::new(config).unwrap(), accept(&listener));
            let (head, body) = request(&mut stream);
            assert!(head.starts_with("POST /v1/live/sessions/live:test%2Fpart/resume "));
            assert_eq!(body["envelope"]["sequence"], 10);
            assert_eq!(body["envelope"]["messageType"], "resume_request");
            assert_eq!(
                body["envelope"]["payload"],
                serde_json::json!({
                "lastControlSequence":9,"lastFrameId":27,"lastInputSequence":3,"requesterDeviceId":"device-1"})
            );
            let mut value: serde_json::Value = serde_json::from_slice(&snapshot()).unwrap();
            if scenario == 2 {
                value["session"]["viewerDevices"] = serde_json::json!([]);
            }
            reply(
                &mut stream,
                if scenario == 1 { 409 } else { 200 },
                &serde_json::to_vec(&value).unwrap(),
            );
        });
        let baseline = RequesterCursor {
            control: 9,
            input: 3,
        };
        let result = Control::new(&client, &session, &Cancel::new()).resume(
            expected.identity(),
            baseline,
            27,
        );
        match scenario {
            0 => assert_eq!(result.unwrap().control_sequence, 10),
            1 => assert!(matches!(result, Err(LiveError::Status(409)))),
            _ => assert!(matches!(result, Err(LiveError::Protocol(_)))),
        }
        assert_eq!(baseline.control, 9);
        peer.join().unwrap();
    }
}

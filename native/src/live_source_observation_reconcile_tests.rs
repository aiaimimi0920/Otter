//! 未知写入的完整独立快照oracle；旧cursor不授重发资格，错来源/尺寸/载荷不授成功。
use super::*;
#[test]
fn source_observation_reconcile_rejects_payload_identity_geometry_and_duplicate_mismatches() {
    for case in 0..10 {
        let (listener, client, tls) = fixture();
        let session = session(&client.origin, u64::MAX);
        let mut publication =
            Publication::new("hook:test", [1, 1], Some(("window:test".into(), 1))).unwrap();
        publication.allow_observation().unwrap();
        let expected = serde_json::to_value(observation()).unwrap();
        let peer = std::thread::spawn(move || {
            let mut snapshot = Value::Null;
            for stage in 0..3 {
                let mut stream = StreamOwned::new(
                    ServerConnection::new(tls.clone()).unwrap(),
                    accept(&listener),
                );
                let (head, body) = request(&mut stream);
                if stage == 0 {
                    snapshot = json!({"session":body["envelope"]["payload"]["session"],"epoch":1,"closed":false,"observations":[],"requesterControl":{"deviceId":"device-1","epoch":1,"controlSequence":1}});
                    reply(&mut stream, 201, &serde_json::to_vec(&snapshot).unwrap());
                } else if stage == 1 {
                    assert!(head.contains("/observations HTTP/1.1"));
                    assert_eq!(body["envelope"]["sequence"], 2);
                    assert_eq!(body["envelope"]["payload"], expected);
                    reply(&mut stream, 500, b"{}");
                } else {
                    assert!(head.starts_with("GET "));
                    snapshot["requesterControl"]["controlSequence"] = json!(2);
                    snapshot["observations"] = json!([expected]);
                    match case {
                        0 => {
                            snapshot["observations"][0]["value"]["value"]["text"] =
                                json!("not-seven")
                        }
                        1 => snapshot["observations"] = json!([expected, expected]),
                        2 => snapshot["session"]["sourceRegion"]["width"] = json!(2),
                        3 => snapshot["epoch"] = json!(2),
                        4 => snapshot["session"]["sourceDeviceId"] = json!("other-device"),
                        5 => snapshot["requesterControl"]["controlSequence"] = json!(3),
                        6 => snapshot["closed"] = json!(true),
                        7 => snapshot["session"]["observationCapabilities"] = json!([]),
                        8 => snapshot["observations"] = json!([]),
                        _ => snapshot["requesterControl"]["deviceId"] = json!("other-device"),
                    }
                    reply(&mut stream, 200, &serde_json::to_vec(&snapshot).unwrap());
                }
            }
        });
        let cancel = Cancel::new();
        let control = Control::new(&client, &session, &cancel);
        control.publish_source(&publication).unwrap();
        let value = observation();
        let mut pending = ObservationSubmission::new(&publication, &value).unwrap();
        assert!(
            control
                .publish_source_observation(&publication, &mut pending)
                .is_err(),
            "case {case}"
        );
        peer.join().unwrap();
    }
}
#[test]
fn source_observation_unknown_submission_can_be_cancelled_without_replaying_post() {
    let (listener, client, tls) = fixture();
    let session = session(&client.origin, u64::MAX);
    let mut publication =
        Publication::new("hook:test", [1, 1], Some(("window:test".into(), 1))).unwrap();
    publication.allow_observation().unwrap();
    let (done, wait) = std::sync::mpsc::channel();
    let peer = std::thread::spawn(move || {
        let mut snapshot = Value::Null;
        for stage in 0..3 {
            let mut stream = StreamOwned::new(
                ServerConnection::new(tls.clone()).unwrap(),
                accept(&listener),
            );
            let (head, body) = request(&mut stream);
            if stage == 0 {
                snapshot = json!({"session":body["envelope"]["payload"]["session"],"epoch":1,"closed":false,"observations":[],"requesterControl":{"deviceId":"device-1","epoch":1,"controlSequence":1}});
                reply(&mut stream, 201, &serde_json::to_vec(&snapshot).unwrap());
            } else if stage == 1 {
                reply(&mut stream, 503, b"{}");
            } else {
                assert!(head.starts_with("GET "));
                reply(&mut stream, 200, &serde_json::to_vec(&snapshot).unwrap());
            }
        }
        wait.recv_timeout(Duration::from_secs(2)).unwrap();
        assert!(matches!(listener.accept(),Err(e) if e.kind()==std::io::ErrorKind::WouldBlock));
    });
    let cancel = Cancel::new();
    let control = Control::new(&client, &session, &cancel);
    control.publish_source(&publication).unwrap();
    let value = observation();
    let mut pending = ObservationSubmission::new(&publication, &value).unwrap();
    assert_eq!(
        control.publish_source_observation(&publication, &mut pending),
        Ok(false)
    );
    cancel.cancel();
    assert!(control
        .publish_source_observation(&publication, &mut pending)
        .is_err());
    done.send(()).unwrap();
    peer.join().unwrap();
}

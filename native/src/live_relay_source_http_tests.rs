//! 独立检查Source wire合同与错来源响应；实际TLS，不联系真实Loom。
use super::*;
use crate::live_relay_http::source::Publication;
use serde_json::json;
#[test]
fn source_create_and_close_keep_identity_nonce_dimensions_and_no_remote_input() {
    let (listener, client, tls) = fixture();
    let session = session(&client.origin, u64::MAX);
    let publication = Publication::new("hook:test", [3, 2], None).unwrap();
    let id = publication.session().to_owned();
    let peer = std::thread::spawn(move || {
        let mut stream = StreamOwned::new(
            ServerConnection::new(tls.clone()).unwrap(),
            accept(&listener),
        );
        let (head, body) = request(&mut stream);
        assert!(head.starts_with("POST /v1/live/sessions HTTP/1.1"));
        assert!(head
            .to_ascii_lowercase()
            .contains("authorization: device fixture-secret"));
        assert!(body["surfaceInstanceId"].is_null() && body["sourceAttachmentId"].is_null());
        let source = &body["envelope"]["payload"]["session"];
        assert_eq!(source["sessionId"], id);
        assert_eq!(source["sourceDeviceId"], "device-1");
        assert_eq!(source["sourceHookId"], "hook:test");
        assert_eq!(source["sourceKind"], "region");
        assert_eq!(source["interactionCapabilities"], json!([]));
        assert_eq!(source["observationCapabilities"], json!([]));
        assert_eq!(source["frameStream"]["codec"], "raw_bgra");
        assert_eq!(source["frameStream"]["width"], 3);
        assert_eq!(source["frameStream"]["height"], 2);
        let first = head
            .lines()
            .find(|v| v.to_ascii_lowercase().starts_with("x-loom-device-nonce:"))
            .unwrap()
            .to_owned();
        let mut response = json!({"session":source,"epoch":1,"closed":false});
        reply(&mut stream, 201, &serde_json::to_vec(&response).unwrap());
        drop(stream);
        let mut stream = StreamOwned::new(ServerConnection::new(tls).unwrap(), accept(&listener));
        let (head, body) = request(&mut stream);
        assert!(head.contains("/close HTTP/1.1"));
        assert!(!head.contains(&first));
        assert_eq!(body["envelope"]["messageType"], "session_end");
        assert_eq!(body["envelope"]["sequence"], 2);
        assert_eq!(body["envelope"]["payload"]["endedByDeviceId"], "device-1");
        response["closed"] = json!(true);
        reply(&mut stream, 200, &serde_json::to_vec(&response).unwrap());
    });
    let cancel = Cancel::default();
    let control = Control::new(&client, &session, &cancel);
    control.publish_source(&publication).unwrap();
    control.close_source(&publication).unwrap();
    peer.join().unwrap();
}
#[test]
fn source_response_rejects_substituted_identity_geometry_capability_and_close() {
    for mutation in 0..6 {
        let (listener, client, tls) = fixture();
        let session = session(&client.origin, u64::MAX);
        let publication =
            Publication::new("hook:test", [1, 1], Some(("window:test".into(), 42))).unwrap();
        let peer = std::thread::spawn(move || {
            let mut stream =
                StreamOwned::new(ServerConnection::new(tls).unwrap(), accept(&listener));
            let (_, body) = request(&mut stream);
            let mut source = body["envelope"]["payload"]["session"].clone();
            match mutation {
                0 => source["sourceDeviceId"] = json!("other"),
                1 => source["sourceHookId"] = json!("other"),
                2 => source["frameStream"]["width"] = json!(2),
                3 => source["interactionCapabilities"] = json!(["keyboard"]),
                4 => source["sourceWindowIdentity"]["processId"] = json!(43),
                _ => {}
            }
            let response =
                json!({"session":source,"epoch":if mutation==5 {2}else{1},"closed":false});
            reply(&mut stream, 201, &serde_json::to_vec(&response).unwrap());
        });
        let cancel = Cancel::default();
        assert!(Control::new(&client, &session, &cancel)
            .publish_source(&publication)
            .is_err());
        peer.join().unwrap();
    }
    for size in [[0, 1], [16385, 1], [8192, 8192]] {
        assert!(Publication::new("hook:test", size, None).is_err());
    }
    assert!(Publication::new("bad?hook", [1, 1], None).is_err());
    let a = Publication::new("hook:test", [1, 1], None).unwrap();
    let b = Publication::new("hook:test", [1, 1], None).unwrap();
    assert_ne!(a.session(), b.session());
}
#[test]
fn authenticated_source_progress_rejects_substitution_ahead_closed_and_missing_metadata() {
    for mode in 0..9 {
        let (listener, client, tls) = fixture();
        let session = session(&client.origin, u64::MAX);
        let publication = Publication::new("hook:test", [3, 2], None).unwrap();
        let peer = std::thread::spawn(move || {
            let mut response = serde_json::Value::Null;
            for stage in 0..2 {
                let mut stream = StreamOwned::new(
                    ServerConnection::new(tls.clone()).unwrap(),
                    accept(&listener),
                );
                let (head, body) = request(&mut stream);
                if stage == 0 {
                    assert!(head.starts_with("POST "));
                    response = json!({"session":body["envelope"]["payload"]["session"],"epoch":1,"closed":false,"lastFrameId":7,"sourceConnected":true});
                } else {
                    assert!(head.starts_with("GET "));
                    match mode {
                        1 => response["lastFrameId"] = json!(6),
                        2 => response["sourceConnected"] = json!(false),
                        3 => response["session"]["sourceHookId"] = json!("other"),
                        4 => response["session"]["frameStream"]["width"] = json!(4),
                        5 => response["epoch"] = json!(2),
                        6 => response["lastFrameId"] = json!(8),
                        7 => response["closed"] = json!(true),
                        8 => response["sourceConnected"] = serde_json::Value::Null,
                        _ => {}
                    }
                }
                reply(&mut stream, 200, &serde_json::to_vec(&response).unwrap());
            }
        });
        let cancel = Cancel::default();
        let control = Control::new(&client, &session, &cancel);
        control.publish_source(&publication).unwrap();
        let result = control.source_progress(&publication, 7);
        match mode {
            0 => assert_eq!(result, Ok(Some(7))),
            1 | 2 => assert_eq!(result, Ok(None)),
            _ => assert!(result.is_err()),
        }
        peer.join().unwrap();
    }
}

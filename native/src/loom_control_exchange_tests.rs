//! 取消与释放控制消息在真实 TLS/WS 回环上的回执确认。
use super::*;
use crate::{graph_requests::Request, loom_art_control, loom_art_wire, workspace_id::UnitId};
#[test]
fn cancel_and_release_receive_matching_acknowledgements_after_ping() {
    for release in [false, true] {
        let (listener, identity, config) = fixture();
        let request = Request {
            id: UnitId::parse("execution").unwrap(),
            generation: 2,
        };
        let unit = UnitId::parse("unit").unwrap();
        let control_id = if release {
            UnitId::parse("release:fixture").unwrap()
        } else {
            request.id.clone()
        };
        let mut leases = crate::loom_output_leases::Leases::preview(
            unit.clone(),
            request.clone(),
            &serde_json::json!({"kind":"shared_memory","handle":"Loom_Buffer_fixture"}),
        )
        .unwrap();
        let wire = if release {
            leases.begin_release(control_id.clone()).unwrap()
        } else {
            loom_art_wire::cancel(&unit, &request).unwrap()
        };
        let expected_id = control_id.clone();
        let worker = thread::spawn(move || {
            let stream = BridgeStream::new(
                listener.accept().unwrap().0,
                Instant::now() + Duration::from_secs(3),
            )
            .unwrap();
            let mut socket = tungstenite::accept(StreamOwned::new(
                ServerConnection::new(config).unwrap(),
                stream,
            ))
            .unwrap();
            let message = socket.read().unwrap();
            let value: serde_json::Value =
                serde_json::from_str(message.to_text().unwrap()).unwrap();
            assert_eq!(value["params"]["requestId"], expected_id.as_str());
            assert_eq!(
                value["method"],
                if release {
                    "loom.hook.art.resources.release"
                } else {
                    "loom.hook.art.cancel"
                }
            );
            socket.send(tungstenite::Message::Ping(vec![9])).unwrap();
            assert_eq!(socket.read().unwrap(), tungstenite::Message::Pong(vec![9]));
            socket.send(tungstenite::Message::Text(serde_json::json!({"protocolVersion":"loom.hook.v1","requestId":"unrelated","status":"succeeded"}).to_string())).unwrap();
            socket.send(tungstenite::Message::Text(serde_json::json!({"protocolVersion":"loom.hook.v1","requestId":expected_id.as_str(),"status":"succeeded"}).to_string())).unwrap();
        });
        let socket = connect(&identity, Duration::from_secs(3)).unwrap();
        let outcome = loom_art_control::run(socket, wire, &control_id).unwrap();
        assert_eq!(outcome, loom_art_control::Outcome::Succeeded);
        if release {
            assert!(leases.complete_release(&control_id, Some(outcome)));
            assert!(leases.is_empty());
        }
        worker.join().unwrap();
    }
}

#[cfg(windows)]
#[test]
fn delivery_releases_via_pinned_identity_and_retains_failed_decoding_result() {
    let (listener, identity, config) = fixture();
    let worker = thread::spawn(move || {
        let mut previous = String::new();
        for status in ["failed", "succeeded"] {
            let stream = BridgeStream::new(
                listener.accept().unwrap().0,
                Instant::now() + Duration::from_secs(3),
            )
            .unwrap();
            let mut socket = tungstenite::accept(StreamOwned::new(
                ServerConnection::new(config.clone()).unwrap(),
                stream,
            ))
            .unwrap();
            let message = socket.read().unwrap();
            let value: serde_json::Value =
                serde_json::from_str(message.to_text().unwrap()).unwrap();
            assert_eq!(value["method"], "loom.hook.art.resources.release");
            assert_eq!(value["params"]["executionRequestId"], "execution");
            assert_eq!(value["params"]["generation"], 9);
            assert_eq!(
                value["params"]["handles"],
                serde_json::json!(["Loom_Buffer_missing_delivery_fixture"])
            );
            let release_id = value["params"]["requestId"].as_str().unwrap();
            let uuid = release_id.strip_prefix("release:").unwrap();
            assert!(uuid::Uuid::parse_str(uuid).is_ok());
            assert_ne!(release_id, previous);
            previous = release_id.to_owned();
            socket.send(tungstenite::Message::Text(serde_json::json!({"protocolVersion":"loom.hook.v1","requestId":release_id,"status":status}).to_string())).unwrap();
        }
    });
    let request = Request {
        id: UnitId::parse("execution").unwrap(),
        generation: 9,
    };
    let outputs = serde_json::json!({"output_image":{"kind":"shared_memory","handle":"Loom_Buffer_missing_delivery_fixture","format":"rgba8","width":1,"height":1,"size":4}});
    let mut delivery = crate::desktop::loom_delivery::Delivery::prepare(
        UnitId::parse("unit").unwrap(),
        request,
        &outputs,
        None,
        &std::sync::atomic::AtomicBool::new(false),
    )
    .unwrap();
    assert!(delivery.result().is_err());
    assert_eq!(delivery.pending_release_count(), 1);
    assert!(matches!(
        delivery.release_authenticated(&identity),
        Err(crate::desktop::loom_delivery::ReleaseError::Rejected(
            loom_art_control::Outcome::Failed
        ))
    ));
    assert_eq!(delivery.pending_release_count(), 1);
    delivery.release_authenticated(&identity).unwrap();
    assert_eq!(delivery.pending_release_count(), 0);
    assert!(delivery.result().is_err());
    worker.join().unwrap();
}

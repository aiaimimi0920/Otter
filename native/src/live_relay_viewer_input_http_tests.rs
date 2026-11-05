//! 真实隔离HTTPS的独立wire oracle：actor双序号、绑定、nonce及响应错配不容忍。
use super::*;
use crate::{
    live_input::Event,
    live_relay_http::{input::Binding, RequesterCursor},
};
use serde_json::json;
fn identity() -> crate::live_relay_viewer_identity::Identity {
    crate::live_relay_viewer_identity::Snapshot::parse(&snapshot(), "live:test/part")
        .unwrap()
        .identity()
        .clone()
}
fn binding() -> Binding<'static> {
    Binding {
        surface: "surface-1",
        attachment: "attachment-1",
    }
}
#[test]
fn control_and_input_use_distinct_actor_sequences_and_exact_wire() {
    let (listener, client, config) = fixture();
    let session = session(client.origin(), u64::MAX);
    let peer = std::thread::spawn(move || {
        let mut nonces = std::collections::HashSet::new();
        for index in 0..3 {
            let mut stream = StreamOwned::new(
                ServerConnection::new(config.clone()).unwrap(),
                accept(&listener),
            );
            let (head, body) = request(&mut stream);
            let headers = head.to_ascii_lowercase();
            assert!(headers.contains("authorization: device fixture-secret\r\n"));
            let nonce = headers
                .lines()
                .find_map(|l| l.strip_prefix("x-loom-device-nonce: "))
                .unwrap();
            assert!(nonces.insert(nonce.to_string()));
            assert_eq!(body["surfaceInstanceId"], "surface-1");
            assert_eq!(body["attachmentId"], "attachment-1");
            if index == 1 {
                assert!(head.starts_with("POST /v1/live/sessions/live:test%2Fpart/input "));
                assert_eq!(
                    body["envelope"],
                    json!({"protocolVersion":"loom.live.v1","sessionId":"live:test/part","epoch":1,
                    "sequence":3,"messageType":"input_event","payload":{"inputSequence":1,"issuedAtMs":body["envelope"]["payload"]["issuedAtMs"],
                        "sourceDeviceId":"device-1","kind":{"kind":"key","data":{"code":"vk:65","state":"pressed","modifiers":0}}}})
                );
                assert!(body["envelope"]["payload"]["issuedAtMs"].as_u64().unwrap() > 0);
                reply(
                    &mut stream,
                    202,
                    br#"{"protocolVersion":"loom.live.v1","acceptedSequence":9}"#,
                );
            } else {
                assert!(head.starts_with("POST /v1/live/sessions/live:test%2Fpart/control "));
                assert_eq!(
                    body["action"],
                    if index == 0 { "acquire" } else { "release" }
                );
                assert_eq!(body["sequence"], if index == 0 { 2 } else { 4 });
                assert_eq!(body["epoch"], 1);
                assert_eq!(body["leaseDurationMs"], 30000);
                let mut response: serde_json::Value = serde_json::from_slice(&snapshot()).unwrap();
                response["session"]["controllerDevice"] = if index == 0 {
                    json!("device-1")
                } else {
                    serde_json::Value::Null
                };
                reply(&mut stream, 200, &serde_json::to_vec(&response).unwrap());
            }
        }
    });
    let cancel = Cancel::new();
    let control = Control::new(&client, &session, &cancel);
    let id = identity();
    control
        .change_viewer_controller(&id, binding(), 2, true)
        .unwrap();
    assert_eq!(
        control.forward_window_input(
            &id,
            binding(),
            RequesterCursor {
                control: 3,
                input: 1
            },
            Event::Key {
                code: 65,
                down: true
            }
        ),
        Ok(9)
    );
    control
        .change_viewer_controller(&id, binding(), 4, false)
        .unwrap();
    peer.join().unwrap();
}
#[test]
fn substituted_controller_or_input_ack_is_rejected() {
    for bad in [
        json!({"session":{"protocolVersion":"loom.live.v1","sessionId":"live:test/part","sourceDeviceId":"source-1","sourceHookId":"hook-1","revision":2,"viewerDevices":["device-1"],"controllerDevice":"another"},"epoch":1,"closed":false}),
        json!({"protocolVersion":"loom.live.v1","acceptedSequence":0}),
    ] {
        let input = bad.get("acceptedSequence").is_some();
        let (listener, client, config) = fixture();
        let session = session(client.origin(), u64::MAX);
        let peer = std::thread::spawn(move || {
            let mut stream =
                StreamOwned::new(ServerConnection::new(config).unwrap(), accept(&listener));
            request(&mut stream);
            reply(&mut stream, 200, &serde_json::to_vec(&bad).unwrap());
        });
        let cancel = Cancel::new();
        let control = Control::new(&client, &session, &cancel);
        if input {
            assert!(control
                .forward_window_input(
                    &identity(),
                    binding(),
                    RequesterCursor {
                        control: 2,
                        input: 1
                    },
                    Event::Key {
                        code: 65,
                        down: true
                    }
                )
                .is_err());
        } else {
            assert!(control
                .change_viewer_controller(&identity(), binding(), 2, true)
                .is_err());
        }
        peer.join().unwrap();
    }
}

//! 同一串行HTTP owner的释放屏障、actor序列和不隐式接管控制回归。
use super::*;
use crate::device_session_client::tests::{accept, fixture_with_pem, reply, request, session};
use rustls::{ServerConnection, StreamOwned};
use serde_json::json;
fn setup() -> (Arc<Queue>, Identity, RequestTarget) {
    let bytes=serde_json::to_vec(&json!({"session":{"protocolVersion":"loom.live.v1","sessionId":"live:queue",
        "sourceDeviceId":"source-1","sourceHookId":"hook-1","revision":2,"viewerDevices":["device-1"],
        "interactionCapabilities":["keyboard","cancel"]},"epoch":1,"closed":false})).unwrap();
    let id = crate::live_relay_viewer_identity::Snapshot::parse(&bytes, "live:queue")
        .unwrap()
        .identity()
        .clone();
    let media = Arc::new(
        crate::live_relay_viewer_session::ViewerSession::new(id.clone(), "device-1".into())
            .unwrap(),
    );
    let queue = Arc::new(Queue::default());
    queue.configure(
        crate::live_relay_http::input::Capabilities::parse(&bytes).unwrap(),
        &media,
    );
    let target = RequestTarget::Surface(Target {
        session: "live:queue".into(),
        surface: "surface:1".into(),
        attachment: "attachment:1".into(),
        expected: None,
        selected: Some(id.clone()),
    });
    (queue, id, target)
}
#[test]
fn cancel_reenable_serializes_release_before_second_acquire_and_stop() {
    let (listener, client, tls, _) = fixture_with_pem();
    let credential = session(client.origin(), u64::MAX);
    let peer = std::thread::spawn(move || {
        for (index, action) in ["acquire", "release", "acquire", "release"]
            .into_iter()
            .enumerate()
        {
            let mut stream = StreamOwned::new(
                ServerConnection::new(tls.clone()).unwrap(),
                accept(&listener),
            );
            let (head, body) = request(&mut stream);
            assert!(head.starts_with("POST /v1/live/sessions/live:queue/control "));
            assert_eq!(body["action"], action);
            assert_eq!(body["sequence"], index + 2);
            let response = json!({"session":{"protocolVersion":"loom.live.v1","sessionId":"live:queue","sourceDeviceId":"source-1","sourceHookId":"hook-1","revision":index+3,"viewerDevices":["device-1"],"controllerDevice":if action=="acquire"{json!("device-1")}else{serde_json::Value::Null}},"epoch":1,"closed":false});
            reply(&mut stream, 200, &serde_json::to_vec(&response).unwrap());
        }
    });
    let (queue, id, target) = setup();
    let cancel = crate::device_request_cancel::Cancel::new();
    let control = Control::new(&client, &credential, &cancel);
    let mut sender = Sender::new(
        queue.clone(),
        RequesterCursor {
            control: 1,
            input: 0,
        },
    );
    queue.request();
    assert_eq!(sender.drive(&control, &target, &id, 7), Ok(false));
    assert!(queue.status().enabled);
    queue.push(crate::live_input::Event::Key {
        code: 65,
        down: true,
    });
    queue.release();
    queue.request();
    assert_eq!(sender.drive(&control, &target, &id, 7), Ok(false));
    assert!(!queue.status().enabled);
    assert_eq!(sender.drive(&control, &target, &id, 7), Ok(false));
    assert!(queue.status().enabled);
    queue.stop();
    assert_eq!(sender.drive(&control, &target, &id, 7), Ok(true));
    assert!(!queue.cleanup_needed());
    peer.join().unwrap();
}
#[test]
fn existing_device_controller_without_local_intent_never_enables_or_owns_sender() {
    let (queue, _, _) = setup();
    let mut sender = Sender::new(
        queue.clone(),
        RequesterCursor {
            control: 17,
            input: 8,
        },
    );
    sender.observe(true);
    assert!(!sender.owned);
    assert!(!queue.cleanup_needed());
    assert!(!queue.status().enabled);
    assert_eq!(
        sender.cursor,
        RequesterCursor {
            control: 17,
            input: 8
        }
    );
}

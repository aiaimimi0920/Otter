//! 仅合成profile/pinned TLS；真实Application原accept/read/Checkpoint，不设新增生产入口。
use super::*;
use crate::projection_edit::workspace::Snapshot;
use serde_json::Value;
pub(super) fn wire(origin: &str, revision: u64, offline: bool) -> Value {
    let mut value = if offline {
        crate::projection_delivery::tests::offline::response("device-old")
    } else {
        super::super::super::fixture::response(origin)
    };
    value["editing"] = crate::projection_delivery::accept::edit_tests::document(&value, revision);
    value
}
pub(super) fn state(fixture: &Fixture) -> &State {
    &fixture
        .app
        .documents
        .iter()
        .find(|doc| doc.state.unit_id.as_str() == fixture.ticket().unit())
        .unwrap()
        .state
}
pub(super) fn snapshot(fixture: &Fixture) -> Snapshot {
    let state = state(fixture);
    state
        .projection_edit
        .borrow()
        .as_ref()
        .unwrap()
        .capture(&state.canvas.borrow().editor)
}
pub(super) unsafe fn accepted(offline: bool) -> Fixture {
    let mut fixture = Fixture::with_request(
        if offline {
            "/v1/offline-projections/accept"
        } else {
            "/v1/projections/accept"
        },
        move |stream, origin| {
            reply(
                stream,
                200,
                &serde_json::to_vec(&wire(origin, 1, offline)).unwrap(),
            )
        },
    );
    saved(&mut fixture);
    fixture
}
pub(super) fn peer(fixture: &mut Fixture, values: Vec<Value>) -> mpsc::Receiver<()> {
    peer_with_renew(fixture, values, false)
}
pub(super) fn peer_with_renew(
    fixture: &mut Fixture,
    values: Vec<Value>,
    renew: bool,
) -> mpsc::Receiver<()> {
    let listener = fixture.listener.take().unwrap();
    let tls = fixture.tls.clone();
    let offline = fixture.offline;
    let (tx, rx) = mpsc::sync_channel(4);
    fixture.server = Some(std::thread::spawn(move || {
        if renew {
            for index in 0..2 {
                let mut stream = StreamOwned::new(
                    ServerConnection::new(tls.clone()).unwrap(),
                    accept(&listener),
                );
                let (head, _) = request(&mut stream);
                if index == 0 {
                    assert!(head.starts_with("POST /v1/device-sessions/challenges "));
                    reply(
                        &mut stream,
                        200,
                        br#"{"deviceId":"device-old","challengeId":"id","challenge":"value"}"#,
                    );
                } else {
                    assert!(head.starts_with("POST /v1/device-sessions "));
                    let now = crate::desktop::surface_windows::now_ms().unwrap();
                    reply(&mut stream,200,&serde_json::to_vec(&serde_json::json!({"deviceId":"device-old","token":"receive-owner-token","expiresAtMs":now+60_000})).unwrap());
                }
            }
        }
        for value in values {
            let mut stream = StreamOwned::new(
                ServerConnection::new(tls.clone()).unwrap(),
                accept(&listener),
            );
            let (head, body) = request(&mut stream);
            assert!(head.starts_with(if offline {
                "POST /v1/offline-projections/read "
            } else {
                "POST /v1/projections/read "
            }));
            assert!(head
                .to_ascii_lowercase()
                .contains("authorization: device receive-owner-token\r\n"));
            assert!(body["knownRevision"].as_u64().unwrap() >= 5);
            reply(&mut stream, 200, &serde_json::to_vec(&value).unwrap());
            tx.send(()).unwrap();
        }
        listener
    }));
    rx
}
pub(super) unsafe fn save(fixture: &mut Fixture) {
    fixture
        .app
        .start_checkpoint_with(&mut |_, _, message, warning| assert!(!warning, "{message}"));
    checkpoint(&mut fixture.app);
}

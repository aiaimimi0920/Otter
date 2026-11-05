//! 控制 owner 的实际 TLS 撤权与关闭确认；仅访问回环测试服务。
use super::*;
use crate::live_relay_control_owner::{Error as OwnerError, Owner};

fn owner() -> Owner {
    let snapshot =
        crate::live_relay_viewer_identity::Snapshot::parse(&snapshot(), "live:test/part").unwrap();
    let cursor = ControlCursor::new(snapshot.identity().clone(), 6);
    Owner::new(
        crate::live_relay_http::Prepared {
            business:
                crate::live_relay_viewer_business::Business::from_snapshot(&super::snapshot())
                    .unwrap(),
            snapshot,
            cursor,
            control_sequence: 1,
            input_sequence: 0,
        },
        "device-1".into(),
    )
    .unwrap()
}

#[test]
fn authenticated_poll_activates_then_401_revokes_existing_permit() {
    let (listener, client, config) = fixture();
    let session = session(&client.origin, u64::MAX);
    let peer = std::thread::spawn(move || {
        for index in 0..2 {
            let mut stream = StreamOwned::new(
                ServerConnection::new(config.clone()).unwrap(),
                accept(&listener),
            );
            let (head, _) = request(&mut stream);
            assert!(head.contains(if index == 0 { "after=6" } else { "after=7" }));
            if index == 0 {
                reply(&mut stream, 200, &history(false));
            } else {
                reply(&mut stream, 401, b"{}");
            }
        }
    });
    let cancel = Cancel::default();
    let control = Control::new(&client, &session, &cancel);
    let mut owner = owner();
    owner.poll(&control, |_| Ok(())).unwrap();
    let permit = owner.permit().unwrap();
    assert_eq!(
        owner.poll(&control, |_| panic!("401 consumed")),
        Err(OwnerError::Http(LiveError::AuthorizationRequired))
    );
    assert!(owner.media().validate(&permit).is_err());
    assert_eq!(owner.after(), 7);
    peer.join().unwrap();
}

#[test]
fn events_404_requires_authenticated_closed_member_snapshot() {
    for terminal in [false, true] {
        let (listener, client, config) = fixture();
        let session = session(&client.origin, u64::MAX);
        let peer = std::thread::spawn(move || {
            for index in 0..2 {
                let mut stream = StreamOwned::new(
                    ServerConnection::new(config.clone()).unwrap(),
                    accept(&listener),
                );
                let (head, _) = request(&mut stream);
                if index == 0 {
                    assert!(head.contains("/events?"));
                    reply(&mut stream, 404, b"{}");
                } else {
                    assert!(head.starts_with("GET /v1/live/sessions/live:test%2Fpart HTTP/1.1"));
                    let mut value: serde_json::Value = serde_json::from_slice(&snapshot()).unwrap();
                    value["closed"] = serde_json::json!(terminal);
                    reply(&mut stream, 200, &serde_json::to_vec(&value).unwrap());
                }
            }
        });
        let cancel = Cancel::default();
        let control = Control::new(&client, &session, &cancel);
        let mut owner = owner();
        let result = owner.poll(&control, |_| panic!("404 consumed"));
        if terminal {
            assert!(result.unwrap().closed);
        } else {
            assert_eq!(result, Err(OwnerError::Http(LiveError::Status(404))));
        }
        assert!(owner.permit().is_err());
        assert_eq!(owner.after(), 6);
        peer.join().unwrap();
    }
}

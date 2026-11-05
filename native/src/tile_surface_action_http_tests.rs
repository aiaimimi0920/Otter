//! 独立TLS wire oracle核固定路由/Device/nonce/sequence和三种ACK身份；未知写不自动重放。
use super::*;
use crate::{
    device_authorization_lease::Grant,
    tile_surface_actions::{
        tests::{ack_value, confirmation, Fixture},
        Kind,
    },
};
use serde_json::json;
#[test]
fn action_confirmation_cancel_use_exact_wall_bodies_and_fresh_device_nonces() {
    let (listener, client, config) = fixture();
    let mut grant = Grant::new(session(client.origin(), u64::MAX));
    let lease = grant.lease(client.origin(), 0).unwrap();
    let f = Fixture::new("art");
    let mut command = f.commands().pop().unwrap();
    let Kind::Event(event) = &command.kind else {
        panic!()
    };
    let id = event["eventId"].as_str().unwrap().to_owned();
    let expected_view = serde_json::to_value(&f.state.view).unwrap();
    let expected_id = id.clone();
    let peer = std::thread::spawn(move || {
        let mut nonces = std::collections::BTreeSet::new();
        for path in ["event", "confirmation", "cancel"] {
            let mut stream = StreamOwned::new(
                ServerConnection::new(config.clone()).unwrap(),
                accept(&listener),
            );
            let (head, body) = request(&mut stream);
            assert!(head.starts_with(&format!("POST /v1/walls/surfaces/{path} HTTP/1.1")));
            let head = head.to_ascii_lowercase();
            assert!(head.contains("authorization: device fixture-secret\r\n"));
            let nonce = head
                .lines()
                .find(|s| s.starts_with("x-loom-device-nonce:"))
                .unwrap()
                .to_owned();
            assert!(nonces.insert(nonce));
            assert_eq!(body["view"], expected_view);
            match path {
                "event" => {
                    assert_eq!(body["sequence"], 1);
                    assert_eq!(body["placementId"], "interactive");
                    assert_eq!(body["pixel"], json!({"x":0,"y":0}));
                    assert_eq!(body["event"]["eventId"], expected_id);
                    assert_eq!(body["event"]["payload"], json!({"known":7}));
                }
                "confirmation" => {
                    assert_eq!(body["confirmationId"], "confirm");
                    assert_eq!(body["approved"], false);
                    assert!(body.get("sequence").is_none());
                }
                _ => {
                    assert_eq!(body["requestId"], "request");
                    assert_eq!(body.as_object().unwrap().len(), 2);
                }
            }
            reply(
                &mut stream,
                202,
                &serde_json::to_vec(&ack_value(&expected_id, "cancelled")).unwrap(),
            );
        }
    });
    let control = lease.control(&client).unwrap();
    assert!(control
        .wall_surface_command(&command, &f.state)
        .unwrap()
        .accepted());
    let c = serde_json::from_value(confirmation(&id, &f.job.key.endpoint)).unwrap();
    command.kind = Kind::Decision {
        confirmation: c,
        approved: false,
    };
    assert_eq!(
        control
            .wall_surface_command(&command, &f.state)
            .unwrap()
            .status(),
        crate::surface_action_ack::Status::Cancelled
    );
    command.kind = Kind::Cancel {
        request: "request".into(),
        event: id,
    };
    assert!(control
        .wall_surface_command(&command, &f.state)
        .unwrap()
        .accepted());
    peer.join().unwrap();
}
#[test]
fn wrong_ack_or_lost_response_is_not_accepted_or_retried() {
    for mode in ["event", "request", "lost"] {
        let (listener, client, config) = fixture();
        let mut grant = Grant::new(session(client.origin(), u64::MAX));
        let lease = grant.lease(client.origin(), 0).unwrap();
        let f = Fixture::new("art");
        let mut c = f.commands().pop().unwrap();
        c.kind = Kind::Cancel {
            request: "request".into(),
            event: "event".into(),
        };
        let peer = std::thread::spawn(move || {
            let mut stream =
                StreamOwned::new(ServerConnection::new(config).unwrap(), accept(&listener));
            let _ = request(&mut stream);
            if mode != "lost" {
                let mut v = ack_value("event", "cancelled");
                v[if mode == "event" {
                    "eventId"
                } else {
                    "requestId"
                }] = "wrong".into();
                reply(&mut stream, 202, &serde_json::to_vec(&v).unwrap());
            }
            drop(stream);
            std::thread::sleep(Duration::from_millis(100));
            assert!(listener.accept().is_err(), "never retry uncertain mutation");
        });
        assert!(lease
            .control(&client)
            .unwrap()
            .wall_surface_command(&c, &f.state)
            .is_err());
        peer.join().unwrap();
    }
}

//! 实际TLS顺序由独立接收端核验：heartbeat未确认前零Event，之后原事件恰一次且期限不变。
use super::*;
use crate::tile_shared::admission::{permits, Heartbeat};
use std::{sync::mpsc, time::Duration};

fn grant(client: &Client) -> crate::device_authorization_lease::Grant {
    crate::device_authorization_lease::Grant::new(crate::device_session_client::tests::session(
        client.origin(),
        u64::MAX,
    ))
}
fn acknowledge(f: &Fixture) {
    let mut state = f.shared.lock();
    Heartbeat::capture(&mut state, Some(&f.job.key), Some(f.job.key.revision))
        .acknowledge(&mut state);
}
#[test]
fn actual_tls_heartbeat_ack_precedes_the_original_event_without_extending_its_deadline() {
    let (listener, client, config, _) = crate::device_session_client::tests::fixture_with_pem();
    let peer_listener = listener.try_clone().unwrap();
    // Windows克隆可能恢复副本的阻塞模式；无连接断言也必须保有界，不能挂在accept。
    peer_listener.set_nonblocking(true).unwrap();
    let mut grant = grant(&client);
    let presenter_lease = grant.lease(client.origin(), 0).unwrap();
    let art_lease = grant.lease(client.origin(), 0).unwrap();
    let f = Fixture::new("art");
    f.shared.lock().input_applied = None;
    let command = f.commands().pop().unwrap();
    let until = command.until;
    let Kind::Event(event) = &command.kind else {
        panic!()
    };
    let id = event["eventId"].as_str().unwrap().to_owned();
    assert!(f.shared.lock().actions.offer(vec![command], Instant::now()));
    let views = Views::new(&client, &art_lease);
    views.dispatch_state(
        f.state.clone(),
        Some(f.art.clone()),
        &f.shared,
        &f.job,
        false,
    );
    assert!(
        listener.accept().is_err(),
        "paint without service ACK must not open HTTP"
    );
    let (at, arrived) = mpsc::sync_channel(0);
    let (release, go) = mpsc::sync_channel(0);
    std::thread::scope(|scope| {
        let key = &f.job.key;
        let peer = scope.spawn(move || {
            let mut stream = StreamOwned::new(
                ServerConnection::new(config.clone()).unwrap(),
                accept(&peer_listener),
            );
            let (head, body) = request(&mut stream);
            assert!(head.starts_with("POST /v1/walls/heartbeat HTTP/1.1"));
            assert_eq!(body["sequence"], 1);
            assert_eq!(body["appliedRevision"], key.revision);
            at.send(()).unwrap();
            go.recv_timeout(Duration::from_secs(2)).unwrap();
            reply(
                &mut stream,
                200,
                br#"{"protocolVersion":"loom.wall.v1","accepted":true}"#,
            );
            let mut stream = StreamOwned::new(
                ServerConnection::new(config).unwrap(),
                accept(&peer_listener),
            );
            let (head, body) = request(&mut stream);
            assert!(head.starts_with("POST /v1/walls/surfaces/event HTTP/1.1"));
            assert_eq!(body["event"]["eventId"], id);
            assert_eq!(body["event"]["payload"], serde_json::json!({"known":7}));
            assert_eq!(body["sequence"], 1);
            reply(
                &mut stream,
                202,
                &serde_json::to_vec(&crate::tile_surface_actions::tests::ack_value(
                    &id,
                    "succeeded",
                ))
                .unwrap(),
            );
            assert!(peer_listener.accept().is_err(), "one event only; no replay");
        });
        let f = &f;
        let client = &client;
        let presenter = scope.spawn(move || {
            let heartbeat = Heartbeat::capture(&mut f.shared.lock(), Some(key), Some(key.revision));
            presenter_lease
                .control(client)
                .unwrap()
                .wall_heartbeat(&key.endpoint, &key.lease, 1, Some(key.revision), None, None)
                .unwrap();
            heartbeat.acknowledge(&mut f.shared.lock());
        });
        arrived.recv_timeout(Duration::from_secs(2)).unwrap();
        views.dispatch_state(
            f.state.clone(),
            Some(f.art.clone()),
            &f.shared,
            &f.job,
            false,
        );
        assert!(
            listener.accept().is_err(),
            "in-flight heartbeat is not an ACK"
        );
        assert_eq!(
            f.shared.lock().actions.pending.front().unwrap().until,
            until
        );
        assert!(
            Instant::now() < until,
            "do not extend the original 1500ms intent"
        );
        release.send(()).unwrap();
        presenter.join().unwrap();
        assert!(permits(&f.shared.lock(), &f.job.key));
        views.dispatch_state(
            f.state.clone(),
            Some(f.art.clone()),
            &f.shared,
            &f.job,
            false,
        );
        assert!(f.shared.lock().actions.pending.is_empty());
        peer.join().unwrap();
    });
}
#[test]
fn waiting_for_service_ack_keeps_original_deadline_and_never_revives_expired_intent() {
    let (listener, client, _, _) = crate::device_session_client::tests::fixture_with_pem();
    let mut grant = grant(&client);
    let lease = grant.lease(client.origin(), 0).unwrap();
    let f = Fixture::new("art");
    f.shared.lock().input_applied = None;
    let command = f.commands().pop().unwrap();
    let until = command.until;
    assert!(f.shared.lock().actions.offer(vec![command], Instant::now()));
    let views = Views::new(&client, &lease);
    for _ in 0..3 {
        views.dispatch_state(
            f.state.clone(),
            Some(f.art.clone()),
            &f.shared,
            &f.job,
            false,
        );
        assert_eq!(
            f.shared.lock().actions.pending.front().unwrap().until,
            until
        );
        assert!(listener.accept().is_err());
    }
    assert!(f.shared.lock().actions.take(&f.state, until).is_none());
    acknowledge(&f);
    views.dispatch_state(
        f.state.clone(),
        Some(f.art.clone()),
        &f.shared,
        &f.job,
        false,
    );
    assert!(f.shared.lock().actions.pending.is_empty());
    assert!(
        listener.accept().is_err(),
        "late ACK cannot resurrect an expired event"
    );
}
#[test]
fn service_ack_revoked_during_prepare_defers_once_and_fresh_ack_releases_original_event() {
    let (listener, client, config, _) = crate::device_session_client::tests::fixture_with_pem();
    let mut grant = grant(&client);
    let lease = grant.lease(client.origin(), 0).unwrap();
    let f = Fixture::new("art");
    let command = f.commands().pop().unwrap();
    let until = command.until;
    assert!(f.shared.lock().actions.offer(vec![command], Instant::now()));
    let shared = f.shared.clone();
    HOOK.with(|h| *h.borrow_mut() = Some(Box::new(move |_| shared.lock().input_applied = None)));
    let views = Views::new(&client, &lease);
    views.dispatch_state(
        f.state.clone(),
        Some(f.art.clone()),
        &f.shared,
        &f.job,
        false,
    );
    assert!(
        listener.accept().is_err(),
        "write fence must recheck the service ACK"
    );
    assert_eq!(
        f.shared.lock().actions.pending.front().unwrap().until,
        until
    );
    acknowledge(&f);
    let peer = std::thread::spawn(move || {
        let mut stream =
            StreamOwned::new(ServerConnection::new(config).unwrap(), accept(&listener));
        let (head, body) = request(&mut stream);
        assert!(head.starts_with("POST /v1/walls/surfaces/event HTTP/1.1"));
        let id = body["event"]["eventId"].as_str().unwrap();
        reply(
            &mut stream,
            202,
            &serde_json::to_vec(&crate::tile_surface_actions::tests::ack_value(
                id,
                "succeeded",
            ))
            .unwrap(),
        );
        assert!(listener.accept().is_err());
    });
    views.dispatch_state(
        f.state.clone(),
        Some(f.art.clone()),
        &f.shared,
        &f.job,
        false,
    );
    assert!(f.shared.lock().actions.pending.is_empty());
    peer.join().unwrap();
}
#[test]
fn layout_ack_does_not_replace_actual_paint_or_require_reack_for_every_art_redraw() {
    let f = Fixture::new("art");
    assert!(permits(&f.shared.lock(), &f.job.key));
    f.shared
        .unavailable(f.job.generation, &f.job.key, "new Art pixels");
    assert!(permits(&f.shared.lock(), &f.job.key));
    assert!(!crate::tile_surface_action_dispatch::ready(
        &f.shared.lock(),
        &f.state,
        &f.job,
        Instant::now()
    ));
}

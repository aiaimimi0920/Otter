//! 在真实prepare之后用同步屏障撤销；HTTP计数由独立TLS接收端观察，不靠生产自报。
use super::*;
use std::{sync::mpsc, time::Duration};

fn grant(client: &Client) -> crate::device_authorization_lease::Grant {
    crate::device_authorization_lease::Grant::new(crate::device_session_client::tests::session(
        client.origin(),
        u64::MAX,
    ))
}
#[test]
fn lifecycle_changes_after_expensive_prepare_never_start_http_or_revive_retired_intents() {
    for mode in [
        "stop",
        "clear-resume",
        "suspend-resume",
        "freeze",
        "black",
        "generation",
        "expire",
        "deadline",
        "lease",
    ] {
        let (listener, client, _, _) = crate::device_session_client::tests::fixture_with_pem();
        let mut grant = grant(&client);
        let lease = grant.lease(client.origin(), 0).unwrap();
        let f = Fixture::new("art");
        assert!(f.shared.lock().actions.offer(f.commands(), Instant::now()));
        let (at, prepared) = mpsc::sync_channel(0);
        let (resume, go) = mpsc::sync_channel(0);
        std::thread::scope(|scope| {
            let f = &f;
            let client = &client;
            let lease = &lease;
            let worker = scope.spawn(move || {
                HOOK.with(|h| {
                    *h.borrow_mut() = Some(Box::new(move |c| {
                        if mode == "deadline" {
                            c.until = Instant::now();
                        }
                        at.send(()).unwrap();
                        go.recv_timeout(Duration::from_secs(2)).unwrap();
                    }))
                });
                Views::new(client, lease).dispatch_state(
                    f.state.clone(),
                    Some(f.art.clone()),
                    &f.shared,
                    &f.job,
                    false,
                );
            });
            prepared
                .recv_timeout(Duration::from_secs(2))
                .expect("actual prepare reached");
            {
                let mut s = f.shared.lock();
                match mode {
                    "stop" => {
                        drop(s);
                        f.shared.stop();
                    }
                    "clear-resume" => {
                        s.actions.clear();
                        s.actions.observe(
                            f.state.clone(),
                            client.origin(),
                            "device",
                            Instant::now(),
                        );
                    }
                    "suspend-resume" => s.actions.suspend(),
                    "freeze" => {
                        s.frozen = true;
                        s.actions.suspend();
                    }
                    "black" => {
                        s.black = true;
                        s.actions.suspend();
                    }
                    "generation" => s.generation += 1,
                    "expire" => {
                        let until = s.until.unwrap();
                        s.expire(until);
                    }
                    "deadline" => (),
                    "lease" => lease.revoke(),
                    _ => unreachable!(),
                }
            }
            resume.send(()).unwrap();
            worker.join().unwrap();
        });
        assert!(
            listener.accept().is_err(),
            "{mode}: no socket/POST before commit"
        );
        assert!(
            f.shared.lock().actions.pending.is_empty(),
            "{mode}: no resurrection"
        );
    }
}
#[test]
fn redraw_during_prepare_defers_original_once_without_extending_deadline() {
    let (listener, client, config, _) = crate::device_session_client::tests::fixture_with_pem();
    let mut grant = grant(&client);
    let lease = grant.lease(client.origin(), 0).unwrap();
    let f = Fixture::new("art");
    let command = f.commands().pop().unwrap();
    let until = command.until;
    assert!(f.shared.lock().actions.offer(vec![command], Instant::now()));
    let shared = f.shared.clone();
    let job = f.job.clone();
    let frame = f.frame.clone();
    HOOK.with(|h| {
        *h.borrow_mut() = Some(Box::new(move |_| {
            shared.unavailable(job.generation, &job.key, "prepare gap");
            shared.publish(
                job.generation,
                crate::tile_shared::Frame {
                    key: job.key,
                    layout: job.layout,
                    size: job.size,
                    images: frame.images.clone(),
                    arts: frame.arts.clone(),
                    live_until: None,
                },
            );
        }))
    });
    let views = Views::new(&client, &lease);
    views.dispatch_state(
        f.state.clone(),
        Some(f.art.clone()),
        &f.shared,
        &f.job,
        false,
    );
    assert!(listener.accept().is_err());
    assert_eq!(f.shared.lock().actions.pending.len(), 1);
    assert_eq!(
        f.shared.lock().actions.pending.front().unwrap().until,
        until
    );
    let frame = f.shared.lock().frame.clone().unwrap();
    f.shared.confirm(&frame, Instant::now());
    let peer = std::thread::spawn(move || {
        let mut stream =
            StreamOwned::new(ServerConnection::new(config).unwrap(), accept(&listener));
        let (_, body) = request(&mut stream);
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
fn cancel_still_sends_once_when_frozen_after_prepare() {
    let (listener, client, config, _) = crate::device_session_client::tests::fixture_with_pem();
    let mut grant = grant(&client);
    let lease = grant.lease(client.origin(), 0).unwrap();
    let f = Fixture::new("art");
    let mut wire = value(&f.job.key, "art");
    wire["pending"] = serde_json::json!([{"ack":crate::tile_surface_actions::tests::ack_value("event", "running"),"actionId":"run","cancelable":true}]);
    let state = Arc::new(
        State::parse(
            &serde_json::to_vec(&wire).unwrap(),
            &f.state.view.binding,
            "art",
            Some(&f.state),
        )
        .unwrap(),
    );
    let mut command = f.commands().pop().unwrap();
    command.kind = Kind::Cancel {
        request: "request".into(),
        event: "event".into(),
    };
    assert!(f.shared.lock().actions.offer(vec![command], Instant::now()));
    let shared = f.shared.clone();
    HOOK.with(|h| {
        *h.borrow_mut() = Some(Box::new(move |_| {
            let mut s = shared.lock();
            s.frozen = true;
            s.actions.suspend();
        }))
    });
    let peer = std::thread::spawn(move || {
        let mut stream =
            StreamOwned::new(ServerConnection::new(config).unwrap(), accept(&listener));
        let (head, body) = request(&mut stream);
        assert!(head.starts_with("POST /v1/walls/surfaces/cancel HTTP/1.1"));
        assert_eq!(body["requestId"], "request");
        reply(
            &mut stream,
            202,
            &serde_json::to_vec(&crate::tile_surface_actions::tests::ack_value(
                "event",
                "cancelled",
            ))
            .unwrap(),
        );
        assert!(listener.accept().is_err());
    });
    Views::new(&client, &lease).dispatch_state(state, None, &f.shared, &f.job, true);
    assert_eq!(f.shared.lock().actions.notice, "Art操作被拒绝"); // 原Queue将Cancelled ACK列为拒绝终态。
    assert_eq!(
        f.shared.lock().actions.test_dispatch.as_ref().unwrap()["stage"],
        "http_ack"
    );
    peer.join().unwrap();
}
#[test]
fn lost_http_after_commit_never_returns_to_pending_or_retries() {
    let (listener, client, config, _) = crate::device_session_client::tests::fixture_with_pem();
    let mut grant = grant(&client);
    let lease = grant.lease(client.origin(), 0).unwrap();
    let f = Fixture::new("art");
    assert!(f.shared.lock().actions.offer(f.commands(), Instant::now()));
    let peer = std::thread::spawn(move || {
        let mut stream =
            StreamOwned::new(ServerConnection::new(config).unwrap(), accept(&listener));
        let (head, _) = request(&mut stream);
        assert!(head.starts_with("POST /v1/walls/surfaces/event HTTP/1.1"));
        drop(stream);
        std::thread::sleep(Duration::from_millis(50));
        assert!(listener.accept().is_err(), "unknown write must not replay");
    });
    let views = Views::new(&client, &lease);
    views.dispatch_state(
        f.state.clone(),
        Some(f.art.clone()),
        &f.shared,
        &f.job,
        false,
    );
    assert_eq!(f.shared.lock().actions.notice, "Art操作未确认；未重放");
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

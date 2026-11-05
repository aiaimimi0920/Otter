//! 原worker在publish/paint间的确定性回归；只用隔离TLS，不操作用户窗口。
use super::*;
use crate::device_session_client::tests::{accept, reply, request};
use crate::tile_surface_actions::tests::{value, Fixture};
use rustls::{ServerConnection, StreamOwned};
use std::cell::RefCell;
type Hook = Box<dyn FnOnce(&mut Command)>;
thread_local! { static HOOK: RefCell<Option<Hook>> = RefCell::new(None); }
pub(super) fn prepared(command: &mut Command) {
    let hook = HOOK.with(|h| h.borrow_mut().take());
    if let Some(hook) = hook {
        hook(command);
    }
}
#[path = "tile_surface_dispatch_admission_tests.rs"]
mod admission;
#[path = "tile_surface_dispatch_fence_tests.rs"]
mod fences;
#[path = "tile_surface_scroll_dispatch_tests.rs"]
mod scrolling;

#[test]
fn published_successor_without_paint_keeps_original_intent_and_deadline() {
    let (listener, client, config, _) = crate::device_session_client::tests::fixture_with_pem();
    let mut grant = crate::device_authorization_lease::Grant::new(
        crate::device_session_client::tests::session(client.origin(), u64::MAX),
    );
    let lease = grant.lease(client.origin(), 0).unwrap();
    let f = Fixture::new("art");
    let mut views = Views::new(&client, &lease);
    views.entries.insert(
        "art".into(),
        Entry {
            state: f.state.clone(),
            ready: Some(f.art.clone()),
            next: Instant::now(),
        },
    );
    let command = f.commands().pop().unwrap();
    let until = command.until;
    let Kind::Event(event) = &command.kind else {
        panic!()
    };
    let id = event["eventId"].clone();
    assert!(f.shared.lock().actions.offer(vec![command], Instant::now()));
    let mut wire = value(&f.job.key, "art");
    wire["snapshot"]["revision"] = 2.into();
    let state = State::parse(
        &serde_json::to_vec(&wire).unwrap(),
        &f.state.view.binding,
        "art",
        Some(&f.state),
    )
    .unwrap();
    assert!(admit(&mut views.entries, "art".into(), state, &f.shared, &f.job).is_none());
    views.prepare().unwrap();
    f.shared.publish(
        f.job.generation,
        crate::tile_shared::Frame {
            key: f.job.key.clone(),
            layout: f.job.layout.clone(),
            size: f.job.size,
            images: Default::default(),
            arts: views.frames(),
            live_until: None,
        },
    );
    assert!(f.shared.lock().visible(Instant::now()));
    assert!(f.shared.lock().presented.is_none());
    views.dispatch("art", &f.shared, &f.job, false);
    views.dispatch("art", &f.shared, &f.job, false);
    assert!(listener.accept().is_err(), "no POST before actual paint");
    {
        let s = f.shared.lock();
        assert_eq!(
            s.actions.pending.len(),
            1,
            "unpainted frame must not consume the intent"
        );
        assert_eq!(s.actions.pending.front().unwrap().until, until);
    }
    let frame = f.shared.lock().frame.clone().unwrap();
    f.shared.confirm(&frame, Instant::now());
    let peer = std::thread::spawn(move || {
        let mut stream =
            StreamOwned::new(ServerConnection::new(config).unwrap(), accept(&listener));
        let (head, body) = request(&mut stream);
        assert!(head.starts_with("POST /v1/walls/surfaces/event HTTP/1.1"));
        assert_eq!(body["event"]["eventId"], id);
        assert_eq!(body["event"]["baseRevision"], 2);
        assert_eq!(body["event"]["payload"], serde_json::json!({"known":7}));
        reply(
            &mut stream,
            202,
            &serde_json::to_vec(&crate::tile_surface_actions::tests::ack_value(
                id.as_str().unwrap(),
                "succeeded",
            ))
            .unwrap(),
        );
        assert!(listener.accept().is_err(), "exactly one HTTP attempt");
    });
    views.dispatch("art", &f.shared, &f.job, false);
    assert!(f.shared.lock().actions.pending.is_empty());
    assert_eq!(f.shared.lock().actions.notice, "Art操作已受理");
    views.dispatch("art", &f.shared, &f.job, false);
    peer.join().unwrap();
    views.entries.clear();
}

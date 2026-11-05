//! 单槽身份围栏与线程/授权准入；同内容不等于同一次准备，旧准备不得复活。
use super::*;
fn empty() -> Arc<Prepared> {
    Arc::new(Prepared {
        frames: BTreeMap::new(),
        pixels: 0,
    })
}
#[test]
fn old_prepared_clone_cannot_republish_after_new_snapshot_or_generation() {
    let (shared, frame) = crate::tile_tests::shared_frame();
    let a = empty();
    shared.lock().art = Some(a.clone());
    shared.publish_prepared(1, frame, None, Some(&a));
    let first = shared.lock().frame.clone().unwrap();
    {
        let mut s = shared.lock();
        s.art = Some(empty());
        s.unavailable(1, &first.key, "new Art");
    }
    let candidate = || crate::tile_shared::Frame {
        key: first.key.clone(),
        layout: first.layout.clone(),
        size: first.size,
        images: first.images.clone(),
        arts: first.arts.clone(),
        live_until: None,
    };
    shared.publish_prepared(1, candidate(), None, Some(&a));
    assert!(
        shared.lock().frame.is_none(),
        "same content is not same preparation"
    );
    let b = shared.lock().art.clone().unwrap();
    shared.publish_prepared(1, candidate(), None, Some(&b));
    assert!(shared.lock().frame.is_some());
    shared.lock().clear("new generation");
    assert!(shared.lock().art.is_none());
    shared.publish_prepared(1, candidate(), None, Some(&b));
    assert!(shared.lock().frame.is_none());
}
#[test]
fn freeze_and_expiry_do_not_allow_late_worker_to_replace_actual_scene() {
    let (shared, frame) = crate::tile_tests::shared_frame();
    let ready = empty();
    shared.lock().art = Some(ready.clone());
    shared.publish_prepared(1, frame, None, Some(&ready));
    let first = shared.lock().frame.clone().unwrap();
    shared.confirm(&first, std::time::Instant::now());
    shared.lock().frozen = true;
    shared.publish_prepared(
        1,
        crate::tile_shared::Frame {
            key: first.key.clone(),
            layout: first.layout.clone(),
            size: first.size,
            images: BTreeMap::new(),
            arts: BTreeMap::new(),
            live_until: None,
        },
        None,
        Some(&empty()),
    );
    assert!(Arc::ptr_eq(
        shared.lock().presented_frame.as_ref().unwrap(),
        &first
    ));
    let until = shared.lock().until.unwrap();
    shared.lock().expire(until);
    assert!(shared.lock().art.is_none() && shared.lock().frame.is_none());
}
#[test]
fn absent_art_has_no_worker_and_parallel_leases_must_share_original_grant() {
    use crate::device_authorization_lease::Grant;
    let (_listener, client, _, _) = crate::device_session_client::tests::fixture_with_pem();
    let mut grant = Grant::new(crate::device_session_client::tests::session(
        client.origin(),
        u64::MAX,
    ));
    let a = Arc::new(grant.lease(client.origin(), 0).unwrap());
    let (shared, frame) = crate::tile_tests::shared_frame();
    let job = Job {
        generation: 1,
        key: frame.key,
        layout: frame.layout,
        size: frame.size,
    };
    let mut worker = Worker::start(&client, &a, &shared, &job, vec![], 0).unwrap();
    assert!(worker.thread.is_none());
    worker.check().unwrap();
    drop(worker);
    assert!(!a.is_revoked());
    let b = grant.lease(client.origin(), 0).unwrap();
    let c = grant.lease(client.origin(), 0).unwrap();
    let mut other = Grant::new(crate::device_session_client::tests::session(
        client.origin(),
        u64::MAX,
    ));
    let foreign = other.lease(client.origin(), 0).unwrap();
    assert!(crate::tile_runtime::Runtime::test_start(
        client,
        b,
        c,
        foreign,
        crate::tile_tests::output()
    )
    .is_err());
}
#[test]
fn held_worker_releases_shared_lock_and_drains_without_network_or_lease_revocation() {
    use crate::device_authorization_lease::Grant;
    let (listener, client, _, _) = crate::device_session_client::tests::fixture_with_pem();
    let mut grant = Grant::new(crate::device_session_client::tests::session(
        client.origin(),
        u64::MAX,
    ));
    let lease = Arc::new(grant.lease(client.origin(), 0).unwrap());
    let (shared, frame) = crate::tile_tests::shared_frame();
    let job = Job {
        generation: 1,
        key: frame.key,
        layout: frame.layout,
        size: frame.size,
    };
    shared.lock().frozen = true;
    let worker = Worker::start(&client, &lease, &shared, &job, vec!["art".into()], 0).unwrap();
    std::thread::sleep(Duration::from_millis(40));
    assert!(shared.lock().frozen && !shared.lock().stopped);
    drop(worker);
    assert!(!lease.is_revoked() && listener.accept().is_err());
}
#[test]
fn retiring_only_clears_its_own_preparation_and_never_new_generation() {
    let (shared, frame) = crate::tile_tests::shared_frame();
    let job = Job {
        generation: 1,
        key: frame.key,
        layout: frame.layout,
        size: frame.size,
    };
    shared.lock().art = Some(empty());
    drop(Retire {
        shared: &shared,
        job: &job,
    });
    assert!(shared.lock().art.is_none());
    let new = empty();
    {
        let mut s = shared.lock();
        s.generation += 1;
        s.art = Some(new.clone());
    }
    drop(Retire {
        shared: &shared,
        job: &job,
    });
    assert!(Arc::ptr_eq(shared.lock().art.as_ref().unwrap(), &new));
}
#[test]
fn paint_finishing_after_art_replacement_cannot_confirm_old_scene() {
    use serde_json::json;
    let (shared, mut frame) = crate::tile_tests::shared_frame();
    let binding = crate::tile_surface_wire::Binding::from(&frame.key);
    let state = json!({"protocolVersion":"loom.wall.v1","view":{"binding":binding,"instanceId":"art","attachmentId":"view"},"width":2,"height":2,"generation":1,"sequence":0,
        "snapshot":{"protocolVersion":"loom.surface.v1","instanceId":"art","attachmentId":"view","artId":"fixture","artVersion":"1","revision":1,"runtime":"declarative","scene":{"id":"root","type":"view","layout":{"width":2,"height":2}},"resources":[],"resourceLeases":[]},"preview":null,"result":null,"confirmations":[],"pending":[],"failure":null});
    let state = crate::tile_surface_wire::State::parse(
        &serde_json::to_vec(&state).unwrap(),
        &binding,
        "art",
        None,
    )
    .unwrap();
    let make = || {
        Arc::new(Art {
            view: state.view.clone(),
            generation: state.generation,
            snapshot: state.snapshot.clone(),
            size: [2, 2],
            images: BTreeMap::new(),
        })
    };
    frame.arts.insert("art".into(), make());
    let old = Arc::new(frame);
    let current = crate::tile_shared::Frame {
        key: old.key.clone(),
        layout: old.layout.clone(),
        size: old.size,
        images: BTreeMap::new(),
        arts: [("art".into(), make())].into(),
        live_until: None,
    };
    shared.lock().frame = Some(Arc::new(current));
    shared.confirm(&old, std::time::Instant::now());
    assert!(shared.lock().presented_frame.is_none());
    let new = shared.lock().frame.clone().unwrap();
    shared.confirm(&new, std::time::Instant::now());
    assert!(Arc::ptr_eq(
        shared.lock().presented_frame.as_ref().unwrap(),
        &new
    ));
}

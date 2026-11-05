use super::*;
#[path = "live_relay_business_notice_tests.rs"]
mod business_tests;
#[cfg(windows)]
#[path = "live_relay_frame_notice_tests.rs"]
mod notice_tests;
use crate::{
    live_relay_frame::{Codec, ColorSpace},
    pixels::Pixels,
};
fn snapshot(member: bool, closed: bool) -> Vec<u8> {
    serde_json::to_vec(&serde_json::json!({"session":{"protocolVersion":"loom.live.v1","sessionId":"live:test","sourceDeviceId":"source:test","sourceHookId":"hook:test","revision":1,"viewerDevices":if member {vec!["viewer:test"]} else {vec![]}},"epoch":1,"closed":closed})).unwrap()
}
fn prepared() -> (ViewerSession, Snapshot) {
    let snapshot = Snapshot::parse(&snapshot(true, false), "live:test").unwrap();
    let owner = ViewerSession::new(snapshot.identity().clone(), "viewer:test".into()).unwrap();
    (owner, snapshot)
}
fn frame(id: u64, pixels: Pixels) -> PixelFrame {
    PixelFrame::new(
        Metadata {
            epoch: 1,
            frame_id: id,
            capture_timestamp_ms: 1,
            encode_timestamp_ms: 1,
            width: 1,
            height: 1,
            dropped_frames: 0,
            keyframe: true,
            color_space: ColorSpace::Srgb,
            codec: Codec::RawBgra,
        },
        pixels,
    )
    .unwrap()
}
#[test]
fn activation_replaces_generation_and_foreign_or_old_work_cannot_commit() {
    let (owner, snapshot) = prepared();
    assert!(matches!(owner.admit(1), Err(Error::Suspended)));
    owner.activate(&snapshot).unwrap();
    let old = owner.admit(1).unwrap();
    let (other, _) = prepared();
    other.activate(&snapshot).unwrap();
    assert_eq!(
        other.commit(&old, frame(1, vec![255; 4].into()), false),
        Err(Error::StalePermit)
    );
    owner.suspend().unwrap();
    assert!(old.stop().load(std::sync::atomic::Ordering::Acquire));
    owner.activate(&snapshot).unwrap();
    assert_eq!(
        owner.commit(&old, frame(1, vec![255; 4].into()), false),
        Err(Error::StalePermit)
    );
    let current = owner.admit(1).unwrap();
    assert!(!current.stop().load(std::sync::atomic::Ordering::Acquire));
    assert_eq!(
        owner.commit(&current, frame(1, vec![255; 4].into()), false),
        Ok(Commit::Accepted)
    );
    assert!(matches!(owner.admit(2), Err(Error::Epoch)));
}
#[test]
fn revocation_clears_cached_pixels_and_late_open_cannot_reactivate() {
    let (owner, valid) = prepared();
    owner.activate(&valid).unwrap();
    let permit = owner.admit(1).unwrap();
    let pixels = Pixels::from(vec![255; 4]);
    let weak = pixels.downgrade();
    owner.commit(&permit, frame(1, pixels), false).unwrap();
    let removed = Snapshot::parse(&snapshot(false, false), "live:test").unwrap();
    assert_eq!(owner.activate(&removed), Err(Error::Authority));
    assert!(permit.stop().load(std::sync::atomic::Ordering::Acquire));
    assert!(weak.upgrade().is_none());
    assert!(matches!(owner.admit(1), Err(Error::Closed)));
    assert_eq!(owner.activate(&valid), Err(Error::Closed));
    let (owner, valid) = prepared();
    owner.activate(&valid).unwrap();
    assert_eq!(owner.observe_terminal(&snapshot(true, true)), Ok(true));
    assert_eq!(owner.observe_terminal(&snapshot(true, false)), Ok(true));
    assert_eq!(owner.activate(&valid), Err(Error::Closed));
    assert_eq!(
        owner.commit(&permit, frame(2, vec![255; 4].into()), false),
        Err(Error::Closed)
    );
}
#[test]
fn stale_revision_cannot_replace_a_newer_active_generation() {
    let (owner, old) = prepared();
    let mut value: serde_json::Value = serde_json::from_slice(&snapshot(true, false)).unwrap();
    value["session"]["revision"] = serde_json::json!(2);
    let newer = Snapshot::parse(&serde_json::to_vec(&value).unwrap(), "live:test").unwrap();
    owner.activate(&newer).unwrap();
    let permit = owner.admit(1).unwrap();
    assert_eq!(owner.activate(&old), Err(Error::Authority));
    assert_eq!(
        owner.commit(&permit, frame(1, vec![255; 4].into()), false),
        Ok(Commit::Accepted)
    );
}
#[test]
fn close_racing_commit_leaves_no_cached_pixels() {
    for _ in 0..32 {
        let (owner, snapshot) = prepared();
        owner.activate(&snapshot).unwrap();
        let owner = Arc::new(owner);
        let permit = owner.admit(1).unwrap();
        let pixels = Pixels::from(vec![255; 4]);
        let weak = pixels.downgrade();
        let gate = Arc::new(std::sync::Barrier::new(2));
        let worker = {
            let owner = owner.clone();
            let gate = gate.clone();
            std::thread::spawn(move || {
                gate.wait();
                owner.commit(&permit, frame(1, pixels), false)
            })
        };
        gate.wait();
        owner.close();
        assert!(matches!(
            worker.join().unwrap(),
            Ok(Commit::Accepted) | Err(Error::Closed)
        ));
        assert!(weak.upgrade().is_none());
    }
}

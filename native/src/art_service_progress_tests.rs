use super::*;
use serde_json::json;
use std::sync::atomic::{AtomicUsize, Ordering};

fn reply(percent: serde_json::Value) -> Reply {
    serde_json::from_value(json!({
        "protocolVersion":"loom.art.local.service.v1",
        "owner":{"binding":{"origin":"https://localhost","requestId":"request:1",
            "deviceId":"device:1","workspaceId":"workspace:1","instanceId":"instance:1",
            "generation":1,"artId":"vendor/art","artVersion":"1.0.0",
            "packageDigest":"a".repeat(64),"sdkVersion":"otter.art.v1"},
            "grantId":"grant:1","callId":"call:1","service":"capability:vendor:service.run"},
        "sequence":1,"status":"running","error":null,"progress":{"percent":percent,"text":"not exposed"}
    })).unwrap()
}
#[test]
fn service_progress_coalesces_bursts_and_never_wakes_under_lock() {
    let wakes = Arc::new(AtomicUsize::new(0));
    let count = wakes.clone();
    let channel = Channel::new(Arc::new(move || {
        count.fetch_add(1, Ordering::SeqCst);
    }));
    for value in 0..=100 {
        channel.publish(&reply(json!(value)));
    }
    assert_eq!(wakes.load(Ordering::SeqCst), 1);
    assert_eq!(channel.take().unwrap().percent, Some(100));
    channel.publish(&reply(json!(100)));
    assert!(channel.take().is_none());
    assert_eq!(wakes.load(Ordering::SeqCst), 1);
    let received = Arc::new(AtomicUsize::new(usize::MAX));
    let channel: Arc<Channel> = Arc::new_cyclic(|weak: &std::sync::Weak<Channel>| {
        let weak = weak.clone();
        let received = received.clone();
        Channel::new(Arc::new(move || {
            // 若回归到持锁调用，try_lock 立即失败；断言留在 catch_unwind 之外。
            let Some(channel) = weak.upgrade() else {
                return;
            };
            if channel.state.try_lock().is_err() {
                return;
            }
            if let Some(percent) = channel.take().and_then(|update| update.percent) {
                received.store(usize::from(percent), Ordering::SeqCst);
            }
        }))
    });
    channel.publish(&reply(json!(50)));
    assert_eq!(received.load(Ordering::SeqCst), 50);
    assert!(channel.take().is_none());
}
#[test]
fn service_progress_bounds_percent_and_does_not_forward_arbitrary_json() {
    for percent in [
        json!(-1),
        json!(101),
        json!("50"),
        json!(null),
        json!({"secret":"x"}),
    ] {
        assert_eq!(Update::from_reply(&reply(percent)).percent, None);
    }
    assert_eq!(Update::from_reply(&reply(json!(50.9))).percent, Some(50));
}
#[test]
fn service_progress_close_drops_pending_and_rejects_late_updates() {
    let channel = Channel::new(Arc::new(|| {}));
    channel.publish(&reply(json!(25)));
    channel.clear();
    assert!(channel.take().is_none());
    channel.publish(&reply(json!(25)));
    assert!(channel.take().is_some());
    channel.close();
    channel.publish(&reply(json!(75)));
    channel.clear();
    channel.publish(&reply(json!(100)));
    assert!(channel.take().is_none());
}

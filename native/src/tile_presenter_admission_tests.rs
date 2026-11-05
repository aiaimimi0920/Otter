//! 服务端布局ACK与UI paint是两个独立事实；旧回执不能跨完整Key或生命周期生效。
use super::*;
use std::{sync::Arc, time::Instant};

fn fixture() -> (Arc<crate::tile_shared::Shared>, Key) {
    let (shared, frame) = crate::tile_tests::shared_frame();
    let key = frame.key.clone();
    shared.publish(1, frame);
    let frame = shared.lock().frame.clone().unwrap();
    shared.confirm(&frame, Instant::now());
    (shared, key)
}
fn successful(state: &mut State, key: &Key) {
    Heartbeat::capture(state, Some(key), Some(key.revision)).acknowledge(state);
}
#[test]
fn paint_capture_and_dropped_heartbeat_do_not_acknowledge_the_service_layout() {
    let (shared, key) = fixture();
    let mut state = shared.lock();
    assert!(state.presented.is_some());
    assert!(!permits(&state, &key));
    let pending = Heartbeat::capture(&mut state, Some(&key), Some(key.revision));
    assert!(!permits(&state, &key));
    let _ = pending; // HTTP失去回复/未发出时没有成功回执，不能补发本地确认。
    assert!(!permits(&state, &key));
    successful(&mut state, &key);
    assert!(permits(&state, &key));
}
#[test]
fn clear_expire_and_stop_retire_ack_and_reject_late_success() {
    for mode in ["clear", "expire", "stop"] {
        let (shared, key) = fixture();
        let mut state = shared.lock();
        successful(&mut state, &key);
        let pending = Heartbeat::capture(&mut state, Some(&key), Some(key.revision));
        match mode {
            "clear" => state.clear("replaced"),
            "expire" => {
                let until = state.until.unwrap();
                state.expire(until);
            }
            "stop" => {
                drop(state);
                shared.stop();
                state = shared.lock();
            }
            _ => unreachable!(),
        }
        assert!(state.input_applied.is_none(), "{mode}");
        pending.acknowledge(&mut state);
        assert!(!permits(&state, &key), "{mode}");
    }
}
#[test]
fn stale_success_or_null_ack_cannot_replace_or_retire_a_successor() {
    for mode in [
        "generation",
        "endpoint",
        "lease",
        "wall",
        "revision",
        "clock",
    ] {
        let (shared, key) = fixture();
        let mut state = shared.lock();
        let old = Heartbeat::capture(&mut state, Some(&key), Some(key.revision));
        let old_null = Heartbeat::capture(&mut state, Some(&key), None);
        let mut next = key.clone();
        match mode {
            "generation" => state.generation += 1,
            "endpoint" => next.endpoint.push_str("-new"),
            "lease" => next.lease.push_str("-new"),
            "wall" => next.wall.push_str("-new"),
            "revision" => next.revision += 1,
            "clock" => next.clock.push_str("-new"),
            _ => unreachable!(),
        }
        state.desired = Some(next.clone());
        successful(&mut state, &next);
        assert!(permits(&state, &next), "{mode}");
        if next != key {
            let wrong = Heartbeat::capture(&mut state, Some(&key), None);
            assert!(permits(&state, &next), "{mode}: stale capture");
            wrong.acknowledge(&mut state);
            assert!(permits(&state, &next), "{mode}: stale capture ACK");
        }
        old.acknowledge(&mut state);
        assert!(permits(&state, &next), "{mode}: stale success");
        old_null.acknowledge(&mut state);
        assert!(permits(&state, &next), "{mode}: stale null");
    }
}
#[test]
fn null_heartbeat_retires_ack_before_http_and_new_paint_cannot_restore_it() {
    for reply in [false, true] {
        let (shared, key) = fixture();
        let mut state = shared.lock();
        successful(&mut state, &key);
        // 普通同布局续期可以保留已有成功确认。
        let renewal = Heartbeat::capture(&mut state, Some(&key), Some(key.revision));
        assert!(permits(&state, &key));
        let _ = renewal;
        let frame = state.presented_frame.take().unwrap();
        state.presented = None;
        let pending = Heartbeat::capture(&mut state, Some(&key), None);
        assert!(
            !permits(&state, &key),
            "retire before the service can see null"
        );
        drop(state);
        shared.confirm(&frame, Instant::now());
        let mut state = shared.lock();
        assert!(state.presented.is_some(), "local repaint succeeded");
        assert!(!permits(&state, &key), "paint is not a service ACK");
        if reply {
            pending.acknowledge(&mut state);
        } else {
            let _ = pending;
        }
        assert!(
            !permits(&state, &key),
            "null or lost reply cannot restore ACK"
        );
        successful(&mut state, &key);
        assert!(permits(&state, &key));
    }
}
#[test]
fn null_wrong_revision_black_and_frozen_ack_never_grant_input() {
    for mode in ["null", "revision", "black", "frozen"] {
        let (shared, key) = fixture();
        let mut state = shared.lock();
        successful(&mut state, &key);
        let applied = match mode {
            "null" => None,
            "revision" => Some(key.revision + 1),
            "black" => {
                state.black = true;
                Some(key.revision)
            }
            "frozen" => {
                state.frozen = true;
                Some(key.revision)
            }
            _ => unreachable!(),
        };
        Heartbeat::capture(&mut state, Some(&key), applied).acknowledge(&mut state);
        assert!(!permits(&state, &key), "{mode}");
        state.black = false;
        state.frozen = false;
        assert!(!permits(&state, &key), "{mode}: resume cannot invent ACK");
        successful(&mut state, &key);
        assert!(permits(&state, &key));
    }
}

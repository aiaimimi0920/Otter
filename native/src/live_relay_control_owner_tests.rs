use super::*;
use serde_json::{json, Value};

fn owner() -> Owner {
    let bytes = serde_json::to_vec(&json!({"session":{"protocolVersion":"loom.live.v1",
        "sessionId":"live:test","sourceDeviceId":"source:test","sourceHookId":"hook:test",
        "revision":2,"viewerDevices":["viewer:test"],"observationCapabilities":[]},"epoch":1,"closed":false,"observations":[]}))
    .unwrap();
    let snapshot = Snapshot::parse(&bytes, "live:test").unwrap();
    let cursor = ControlCursor::new(snapshot.identity().clone(), 6);
    Owner::new(
        Prepared {
            business: crate::live_relay_viewer_business::Business::from_snapshot(&bytes).unwrap(),
            snapshot,
            cursor,
            control_sequence: 1,
            input_sequence: 0,
        },
        "viewer:test".into(),
    )
    .unwrap()
}

#[test]
fn business_rejection_closes_media_without_advancing_cursor_or_partial_state() {
    for reject_consumer in [false, true] {
        let mut owner = owner();
        activate(&mut owner);
        let media = owner.media();
        let permit = owner.permit().unwrap();
        let mut events = vec![
            json!({"protocolVersion":"loom.live.v1","sessionId":"live:test",
            "epoch":1,"sequence":8,"messageType":"observation","payload":{
                "observationId":"o","sequence":1,"state":"stable","source":"app_adapter",
                "confidence":"high","observedAtMs":1,"value":1}}),
        ];
        if !reject_consumer {
            events.push(
                json!({"protocolVersion":"loom.live.v1","sessionId":"live:test",
                "epoch":1,"sequence":9,"messageType":"trigger_event","payload":{}}),
            );
        }
        let batch = owner.cursor.prepare(&serde_json::to_vec(&json!({
            "protocolVersion":"loom.live.v1","next":7+events.len(),"reset":false,"events":events
        })).unwrap()).unwrap();
        let mut called = false;
        let result = owner.apply(batch, |_| {
            called = true;
            Err(())
        });
        if reject_consumer {
            assert_eq!(result, Err(Error::Consumer));
        } else {
            assert!(matches!(result, Err(Error::Business(_))));
        }
        assert_eq!(called, reject_consumer);
        assert_eq!(owner.after(), 7);
        assert!(owner.business().observations().get("o").is_none());
        assert!(media.validate(&permit).is_err());
    }
}
fn state(revision: u64) -> Value {
    json!({"revision":revision,"visibility":"visible","viewers":["viewer:test"],
        "controllerDeviceId":"viewer:test","reason":"viewer_joined"})
}
fn prepare(owner: &Owner, payloads: Vec<Value>) -> ControlBatch {
    let events: Vec<_> = payloads.into_iter().enumerate().map(|(index, payload)| json!({
        "protocolVersion":"loom.live.v1","sessionId":"live:test","epoch":1,
        "sequence":owner.after()+index as u64+1,"messageType":"session_state","payload":payload
    })).collect();
    owner
        .cursor
        .prepare(
            &serde_json::to_vec(&json!({"protocolVersion":"loom.live.v1",
        "next":owner.after()+events.len() as u64,"reset":false,"events":events}))
            .unwrap(),
        )
        .unwrap()
}
fn activate(owner: &mut Owner) {
    let batch = prepare(owner, vec![state(2)]);
    assert_eq!(
        owner.apply(batch, |_| Ok(())),
        Ok(Applied {
            closed: false,
            controller_owned: true
        })
    );
}
#[test]
fn bootstrap_requires_valid_join_and_consumer_before_activation() {
    let mut owner = owner();
    let media = owner.media();
    assert!(owner.permit().is_err());
    let batch = prepare(&owner, vec![state(2)]);
    owner
        .apply(batch, |events| {
            assert_eq!(events.len(), 1);
            assert!(media.admit(1).is_err());
            Ok(())
        })
        .unwrap();
    assert_eq!(owner.after(), 7);
    assert!(owner.permit().is_ok());
    assert_eq!(
        owner.baseline(),
        RequesterCursor {
            control: 1,
            input: 0
        }
    );
}
#[test]
fn normal_control_update_preserves_frame_cache_and_permit() {
    use crate::{
        live_relay_frame::{Codec, ColorSpace, Metadata},
        live_relay_pixels::PixelFrame,
        pixels::Pixels,
    };
    let mut owner = owner();
    activate(&mut owner);
    let permit = owner.permit().unwrap();
    let pixels = Pixels::from(vec![255; 4]);
    let weak = pixels.downgrade();
    let frame = PixelFrame::new(
        Metadata {
            epoch: 1,
            frame_id: 1,
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
    .unwrap();
    owner.media.commit(&permit, frame, false).unwrap();
    let mut update = state(3);
    update["reason"] = json!("media_disconnected");
    update["controllerDeviceId"] = Value::Null;
    let batch = prepare(&owner, vec![update]);
    assert_eq!(
        owner.apply(batch, |_| Ok(())).unwrap(),
        Applied {
            closed: false,
            controller_owned: false
        }
    );
    assert!(owner.media.validate(&permit).is_ok());
    assert!(weak.upgrade().is_some());
    owner.close();
    assert!(weak.upgrade().is_none());
}
#[test]
fn removal_then_rejoin_is_terminal_before_consumer() {
    let mut owner = owner();
    activate(&mut owner);
    let media = owner.media();
    let permit = owner.permit().unwrap();
    let mut removed = state(3);
    removed["viewers"] = json!([]);
    let batch = prepare(&owner, vec![removed, state(4)]);
    assert!(
        owner
            .apply(batch, |_| {
                assert!(media.validate(&permit).is_err());
                Ok(())
            })
            .unwrap()
            .closed
    );
    assert_eq!(owner.after(), 9);
    assert!(owner.permit().is_err());
}
#[test]
fn malformed_late_state_closes_without_consumption_or_cursor_advance() {
    let mut owner = owner();
    activate(&mut owner);
    let permit = owner.permit().unwrap();
    let mut invalid = state(4);
    invalid["visibility"] = json!("unknown");
    let batch = prepare(&owner, vec![state(3), invalid]);
    assert_eq!(
        owner.apply(batch, |_| panic!("invalid batch consumed")),
        Err(Error::Protocol)
    );
    assert_eq!(owner.after(), 7);
    assert!(owner.media.validate(&permit).is_err());
}
#[test]
fn foreign_and_stale_batches_fail_before_side_effects() {
    let mut target = owner();
    let foreign = owner();
    let batch = prepare(&foreign, vec![state(2)]);
    assert_eq!(
        target.apply(batch, |_| panic!("foreign batch consumed")),
        Err(Error::Protocol)
    );
    let mut target = owner();
    let stale = prepare(&target, vec![state(2)]);
    activate(&mut target);
    assert_eq!(
        target.apply(stale, |_| panic!("stale batch consumed")),
        Err(Error::Protocol)
    );
    assert_eq!(target.after(), 7);
}
#[test]
fn consumer_failure_and_external_suspension_cannot_reactivate() {
    let mut failed = owner();
    let batch = prepare(&failed, vec![state(2)]);
    assert_eq!(failed.apply(batch, |_| Err(())), Err(Error::Consumer));
    assert_eq!(failed.after(), 6);
    assert!(failed.permit().is_err());
    let mut suspended = owner();
    activate(&mut suspended);
    suspended.media.suspend().unwrap();
    let batch = prepare(&suspended, vec![state(3)]);
    assert_eq!(suspended.apply(batch, |_| Ok(())), Err(Error::Media));
    assert_eq!(suspended.after(), 7);
    assert!(suspended.permit().is_err());
}
#[test]
fn closed_visibility_and_empty_bootstrap_fail_safe() {
    let mut closed = owner();
    let mut state = state(2);
    state["visibility"] = json!("closed");
    let batch = prepare(&closed, vec![state]);
    assert!(closed.apply(batch, |_| Ok(())).unwrap().closed);
    assert!(closed.permit().is_err());
    let mut empty = owner();
    let batch = prepare(&empty, vec![]);
    assert_eq!(empty.apply(batch, |_| Ok(())), Err(Error::Protocol));
    assert_eq!(empty.after(), 6);
}

#[test]
fn non_state_events_are_delivered_and_drop_revokes_media() {
    let mut owner = owner();
    activate(&mut owner);
    let media = owner.media();
    let permit = owner.permit().unwrap();
    let bytes = serde_json::to_vec(&json!({"protocolVersion":"loom.live.v1","next":8,
        "reset":false,"events":[{"protocolVersion":"loom.live.v1","sessionId":"live:test",
        "epoch":1,"sequence":8,"messageType":"observation","payload":{
            "observationId":"o","sequence":1,"state":"stable","source":"app_adapter",
            "confidence":"high","observedAtMs":1,"value":1}}]}))
    .unwrap();
    let batch = owner.cursor.prepare(&bytes).unwrap();
    owner
        .apply(batch, |events| {
            assert_eq!(events.len(), 1);
            assert_eq!(events[0].kind(), "observation");
            Ok(())
        })
        .unwrap();
    assert_eq!(owner.after(), 8);
    assert!(media.validate(&permit).is_ok());
    drop(owner);
    assert!(media.validate(&permit).is_err());
}

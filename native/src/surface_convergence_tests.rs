//! 本地流合同：后置刷新、整批判定、身份隔离及终态拒绝，不发送网络。
use super::*;
use crate::{
    surface_action_ack::Ack,
    surface_convergence::{Error, Tracker},
    surface_stream::Batch,
};
#[path = "surface_confirmation_tests.rs"]
mod confirmations;
#[path = "surface_result_commit_tests.rs"]
mod result_commit;
fn batch(after: u64, next: u64, messages: Vec<serde_json::Value>) -> Batch {
    Batch::parse(&serde_json::to_vec(&serde_json::json!({"protocolVersion":"loom.surface-stream.v1","next":next,"reset":false,"messages":messages})).unwrap(), after).unwrap()
}
fn snapshot(revision: u64) -> serde_json::Value {
    serde_json::json!({"method":"loom.surface.snapshot","params":{"generation":7,"snapshot":{
        "protocolVersion":"loom.surface.v1","instanceId":"instance:test","attachmentId":"attachment:test",
        "revision":revision,"scene":{"id":"root","type":"text","props":{"text":"fresh"}}
    }}})
}
fn message_ack(event: &ClickEvent, status: &str) -> serde_json::Value {
    serde_json::json!({"method":"loom.surface.action.ack","params":ack(event, status)})
}
fn initial(event: &ClickEvent, status: &str) -> Ack {
    Ack::parse(&serde_json::to_vec(&ack(event, status)).unwrap(), event).unwrap()
}
#[test]
fn javascript_convergence_runtime_is_bound_to_original_host_event_not_response() {
    for (javascript_event, javascript_snapshot, accepted) in [
        (false, true, false),
        (true, false, false),
        (true, true, true),
    ] {
        let event = if javascript_event {
            event().javascript()
        } else {
            event()
        };
        let mut message = snapshot(58);
        message["params"]["snapshot"]["state"] =
            serde_json::json!({"quote":42,"nested":{"state":"data"}});
        if javascript_snapshot {
            message["params"]["snapshot"]["runtime"] = "javascript".into();
        }
        let mut state = Tracker::new(&event, &initial(&event, "succeeded")).unwrap();
        let result = state.apply(&batch(0, 9, vec![message]));
        assert_eq!(result.is_ok(), accepted);
        assert_eq!(state.take_snapshot().is_some(), accepted);
        if !accepted {
            assert_eq!(
                result,
                Err(Error::Scene(crate::surface_scene::Error::Runtime))
            );
        }
    }
}
#[test]
fn snapshot_business_state_is_not_lifecycle_state_and_lifecycle_types_stay_strict() {
    let event = event();
    for data in [
        serde_json::json!({"quote":42}),
        serde_json::json!(["active"]),
        serde_json::json!(42),
    ] {
        let mut message = snapshot(58);
        message["params"]["snapshot"]["state"] = data.clone();
        let mut state = Tracker::new(&event, &initial(&event, "succeeded")).unwrap();
        state.apply(&batch(0, 9, vec![message])).unwrap();
        assert_eq!(state.take_snapshot().unwrap().revision, 58);
        let message = serde_json::json!({"method":"loom.surface.lifecycle","params":{"event":{
            "protocolVersion":"loom.surface.v1","instanceId":"instance:test","attachmentId":"attachment:test",
            "state":data,"revision":4}}});
        let mut state = Tracker::new(&event, &initial(&event, "succeeded")).unwrap();
        assert_eq!(
            state.apply(&batch(0, 9, vec![snapshot(58), message])),
            Err(Error::Protocol)
        );
        assert!(state.take_snapshot().is_none());
    }
}
#[test]
fn resource_snapshot_convergence_keeps_latest_full_envelope() {
    let event = event();
    let mut state = Tracker::new(&event, &initial(&event, "succeeded")).unwrap();
    let mut messages = Vec::new();
    for revision in [58, 59] {
        let mut message = snapshot(revision);
        message["params"]["snapshot"] = serde_json::from_str(
            crate::surface_resource_snapshot::tests::raw(
                event.binding(),
                revision,
                &format!("lease:{revision}"),
            )
            .get(),
        )
        .unwrap();
        messages.push(message);
    }
    state.apply(&batch(0, 9, messages)).unwrap();
    let snapshot = state.take_snapshot().unwrap();
    assert_eq!(snapshot.revision, 59);
    snapshot.resources.validate(event.binding(), 59).unwrap();
    #[cfg(windows)]
    assert_eq!(
        snapshot
            .resources
            .mount(event.binding(), 59)
            .unwrap()
            .image_resource(&crate::surface_resource_snapshot::tests::id(), 0)
            .unwrap()
            .id(),
        "lease:59"
    );
}
#[test]
fn success_requires_separate_zero_cursor_refresh_and_yields_snapshot_once() {
    let event = event();
    let mut state = Tracker::new(&event, &initial(&event, "running")).unwrap();
    state
        .apply(&batch(
            0,
            8,
            vec![snapshot(57), message_ack(&event, "succeeded")],
        ))
        .unwrap();
    assert!(state.take_snapshot().is_none());
    assert_eq!(state.after(), 0);
    state
        .apply(&batch(
            0,
            9,
            vec![message_ack(&event, "running"), snapshot(58), snapshot(59)],
        ))
        .unwrap();
    assert!(state.ack().succeeded());
    let snapshot = state.take_snapshot().unwrap();
    assert_eq!(snapshot.revision, 59);
    assert_eq!(snapshot.scene.root().id(), "root");
    assert!(state.take_snapshot().is_none());
    assert_eq!(state.apply(&batch(0, 9, vec![])), Err(Error::State));
}
#[test]
fn later_failure_in_refresh_batch_prevents_snapshot_delivery_and_preserves_failure_ack() {
    let event = event();
    let mut state = Tracker::new(&event, &initial(&event, "succeeded")).unwrap();
    assert_eq!(
        state.apply(&batch(
            0,
            9,
            vec![snapshot(58), message_ack(&event, "failed")]
        )),
        Err(Error::Action)
    );
    assert_eq!(state.failure(), Some(Error::Action));
    assert!(state.take_snapshot().is_none());
    assert_eq!(
        state.ack().status(),
        crate::surface_action_ack::Status::Failed
    );
}
#[test]
fn foreign_and_old_snapshots_do_not_satisfy_refresh() {
    let event = event();
    let mut state = Tracker::new(&event, &initial(&event, "succeeded")).unwrap();
    let mut foreign = snapshot(60);
    foreign["params"]["snapshot"]["attachmentId"] = "other".into();
    let mut old = snapshot(60);
    old["params"]["generation"] = 6.into();
    state
        .apply(&batch(0, 9, vec![foreign, old, snapshot(56)]))
        .unwrap();
    assert!(state.take_snapshot().is_none());
    assert_eq!(state.after(), 0);
}
#[test]
fn generation_change_and_disposal_override_earlier_fresh_snapshot() {
    let event = event();
    for (method, expected) in [
        ("loom.surface.generation", Error::Generation),
        ("loom.surface.dispose", Error::Inactive),
    ] {
        let mut state = Tracker::new(&event, &initial(&event, "succeeded")).unwrap();
        let message = serde_json::json!({"method":method,"params":{"instanceId":"instance:test","attachmentId":"attachment:test","generation":8}});
        assert_eq!(
            state.apply(&batch(0, 9, vec![snapshot(58), message])),
            Err(expected)
        );
        assert!(state.take_snapshot().is_none());
    }
}
#[test]
fn nonzero_history_reset_and_wrong_query_cursor_are_terminal() {
    let event = event();
    let mut state = Tracker::new(&event, &initial(&event, "running")).unwrap();
    state.apply(&batch(0, 8, vec![])).unwrap();
    let reset = Batch::parse(
        br#"{"protocolVersion":"loom.surface-stream.v1","next":9,"reset":true}"#,
        8,
    )
    .unwrap();
    assert_eq!(state.apply(&reset), Err(Error::Reset));
    let mut state = Tracker::new(&event, &initial(&event, "running")).unwrap();
    assert_eq!(state.apply(&batch(3, 4, vec![])), Err(Error::Cursor));
}
#[test]
fn current_lifecycle_stop_and_unknown_action_cannot_be_reported_as_ready() {
    let event = event();
    assert!(matches!(
        Tracker::new(&event, &initial(&event, "unknown")),
        Err(Error::Unknown)
    ));
    let mut state = Tracker::new(&event, &initial(&event, "succeeded")).unwrap();
    let message = serde_json::json!({"method":"loom.surface.lifecycle","params":{"event":{
        "protocolVersion":"loom.surface.v1","instanceId":"instance:test","attachmentId":"attachment:test","state":"inactive","revision":4
    }}});
    assert_eq!(
        state.apply(&batch(0, 9, vec![snapshot(58), message])),
        Err(Error::Inactive)
    );
    assert!(state.take_snapshot().is_none());
}

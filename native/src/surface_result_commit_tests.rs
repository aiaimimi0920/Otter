use super::*;
fn result(revision: u64) -> serde_json::Value {
    serde_json::json!({"method":"loom.surface.result","params":{"commit":{
        "protocolVersion":"loom.surface.v1","instanceId":"instance:test","requestId":"request:test",
        "generation":7,"resultRevision":revision,"outputs":{"text":{"kind":"value","value":"done"}},"statePatch":{"ready":true}
    }}})
}
#[test]
fn commit_survives_separate_refresh_and_only_highest_revision_is_delivered_once() {
    let event = event();
    let mut state = Tracker::new(&event, &initial(&event, "running")).unwrap();
    state
        .apply(&batch(
            0,
            8,
            vec![result(9), message_ack(&event, "succeeded")],
        ))
        .unwrap();
    assert!(state.take_snapshot().is_none());
    state
        .apply(&batch(0, 9, vec![result(8), snapshot(58)]))
        .unwrap();
    let commit = state.take_snapshot().unwrap().commit.unwrap();
    assert_eq!(commit.revision(), 9);
    let raw: serde_json::Value = serde_json::from_str(commit.json().get()).unwrap();
    assert_eq!(raw["outputs"]["text"]["value"], "done");
    assert_eq!(raw["statePatch"]["ready"], true);
    assert!(state.take_snapshot().is_none());
}
#[test]
fn foreign_and_old_commits_are_ignored_but_new_generation_and_bad_protocol_fail() {
    for (field, value, rejected) in [
        ("instanceId", serde_json::json!("other"), false),
        ("requestId", serde_json::json!("other"), false),
        ("generation", serde_json::json!(6), false),
        ("generation", serde_json::json!(8), true),
        ("protocolVersion", serde_json::json!("bad"), true),
        ("resultRevision", serde_json::json!(0), true),
    ] {
        let event = event();
        let mut state = Tracker::new(&event, &initial(&event, "succeeded")).unwrap();
        let mut message = result(1);
        message["params"]["commit"][field] = value;
        let applied = state.apply(&batch(0, 1, vec![snapshot(58), message]));
        assert_eq!(applied.is_err(), rejected, "{field}");
        if rejected {
            assert!(state.take_snapshot().is_none());
        } else {
            assert!(state.take_snapshot().unwrap().commit.is_none());
        }
    }
}
#[test]
fn later_failure_discards_result_and_invalid_output_envelopes_fail_closed() {
    let event = event();
    let mut state = Tracker::new(&event, &initial(&event, "succeeded")).unwrap();
    assert!(state
        .apply(&batch(
            0,
            1,
            vec![result(1), snapshot(58), message_ack(&event, "failed")]
        ))
        .is_err());
    assert!(state.take_snapshot().is_none());
    for outputs in [
        serde_json::json!([]),
        serde_json::json!({"x":{"kind":"unknown"}}),
        serde_json::json!({"x":{"kind":"resource","resource":"file:///bad"}}),
        serde_json::Value::Object(
            (0..65)
                .map(|i| {
                    (
                        format!("p{i}"),
                        serde_json::json!({"kind":"value","value":null}),
                    )
                })
                .collect(),
        ),
    ] {
        let mut message = result(1);
        message["params"]["commit"]["outputs"] = outputs;
        let mut state = Tracker::new(&event, &initial(&event, "succeeded")).unwrap();
        assert!(state
            .apply(&batch(0, 1, vec![message, snapshot(58)]))
            .is_err());
    }
}

#[test]
fn commit_budget_and_duplicate_output_keys_are_rejected_before_retention() {
    use crate::surface_result_commit::{Commit, Error as CommitError, MAX_BYTES};
    let event = event();
    let ack = initial(&event, "succeeded");
    let mut value = result(1)["params"]["commit"].clone();
    value["statePatch"] = serde_json::Value::String("x".repeat(MAX_BYTES));
    let raw = serde_json::value::RawValue::from_string(value.to_string()).unwrap();
    assert!(matches!(
        Commit::parse_matching(&raw, &event, &ack),
        Err(CommitError::Limit)
    ));
    let raw = serde_json::value::RawValue::from_string(r#"{"protocolVersion":"loom.surface.v1","instanceId":"instance:test","requestId":"request:test","generation":7,"resultRevision":1,"outputs":{"x":{"kind":"value"},"x":{"kind":"value"}}}"#.into()).unwrap();
    assert!(matches!(
        Commit::parse_matching(&raw, &event, &ack),
        Err(CommitError::Protocol)
    ));
}

use super::*;
use serde_json::{json, Value};
fn snapshot(observations: Vec<Value>, audits: Vec<Value>) -> Business {
    Business::from_snapshot(&serde_json::to_vec(&json!({"session":{"observationCapabilities":["adapter"]},"observations":observations,"triggerAudits":audits})).unwrap()).unwrap()
}
fn observation(id: usize) -> Value {
    json!({"observationId":format!("o:{id}"),"sequence":1,"state":"stable","source":"app_adapter","confidence":"high","observedAtMs":12,"stableSinceMs":10,"value":{"n":7},"reason":"reason"})
}
fn audit(id: usize) -> Value {
    json!({"triggerId":"t","bindingId":"b","conditionRevision":1,"observationId":"o:0","observationSequence":1,"sourceDeviceId":"d","observationSource":"app_adapter","idempotencyKey":format!("key:{id}"),"outcome":"skipped","evaluatedAtMs":20,"authorizedBy":"d","reason":"viewer_offline"})
}
#[test]
fn readonly_projection_preserves_values_states_and_distinct_pending_and_unsupported() {
    assert_eq!(
        render(None),
        format!(
            "{}\r\n",
            crate::ui_text::text("integration-await-observation")
        )
    );
    let b =
        Business::from_snapshot(br#"{"session":{"observationCapabilities":[]},"observations":[]}"#)
            .unwrap();
    assert!(render(Some(&b)).contains(crate::ui_text::text("integration-unsupported")));
    for (state, label) in [
        ("unknown", "integration-unknown"),
        ("detected", "integration-detected"),
        ("observing", "integration-observing"),
        ("stable", "integration-stable"),
        ("triggered", "integration-triggered"),
        ("stale", "integration-stale"),
        ("error", "integration-error"),
    ] {
        let mut o = observation(0);
        o["state"] = json!(state);
        if ["unknown", "stale", "error"].contains(&state) {
            o.as_object_mut().unwrap().remove("value");
        }
        let text = render(Some(&snapshot(vec![o], vec![])));
        assert!(text.contains(&format!(
            "o:0 · {} · {} / {} · #1",
            crate::ui_text::text(label),
            crate::ui_text::text("integration-application-adapter"),
            crate::ui_text::text("integration-risk-high")
        )));
        assert!(text.contains(&crate::ui_text::format(
            "integration-observation-timing",
            &[("sample", 12.into()), ("stable", "10 ms".into())]
        )));
        assert_eq!(
            text.contains(&crate::ui_text::format(
                "integration-observation-value",
                &[("value", "{\"n\":7}".into())]
            )),
            !["unknown", "stale", "error"].contains(&state)
        );
    }
}
#[test]
fn untrusted_text_cannot_hide_fields_and_oversized_values_are_explicitly_clipped() {
    let mut o = observation(0);
    o["reason"] = json!("remote\0\r\n\u{202e}标题");
    o["value"] = json!("😀".repeat(3000));
    let b = snapshot(vec![o], vec![]);
    let text = render(Some(&b));
    assert!(!text.contains('\0'));
    assert!(!text.contains('\u{202e}'));
    assert!(text.contains("remote\\u{0}\\u{d}\\u{a}\\u{202e}标题"));
    assert!(text.contains(crate::ui_text::text("integration-truncated")));
    assert_eq!(
        b.observations()
            .get("o:0")
            .unwrap()
            .value
            .as_ref()
            .unwrap()
            .as_str()
            .unwrap()
            .chars()
            .count(),
        3000
    );
}
#[test]
fn latest_audits_are_bounded_and_reservation_is_not_reported_as_completed_action() {
    let mut audits: Vec<_> = (0..18).map(audit).collect();
    audits[17]["outcome"] = json!("fired");
    audits[17]["reason"] = json!("surface_action_dispatch_reserved");
    let text = render(Some(&snapshot(vec![], audits)));
    assert!(text.contains(&crate::ui_text::format(
        "integration-observation-audits",
        &[("recent", 16.into()), ("count", 18.into())]
    )));
    assert!(!text.contains("key:0\r"));
    assert!(!text.contains("key:1\r"));
    assert!(text.contains("key:2\r"));
    assert!(text.contains("surface_action_dispatch_reserved"));
    assert!(!text.contains(&crate::ui_text::format(
        "integration-observation-request",
        &[("request", "".into())]
    )));
    assert!(text.contains(&format!(
        "b · {}",
        crate::ui_text::text("integration-dispatching")
    )));
}
#[test]
fn maximum_cardinality_and_utf16_budget_do_not_build_an_unbounded_report() {
    let mut values = Vec::new();
    for i in 0..256 {
        let mut o = observation(i);
        o["value"] = json!("😀".repeat(700));
        o["reason"] = json!("x".repeat(512));
        values.push(o);
    }
    let text = render(Some(&snapshot(values, vec![])));
    assert!(text.encode_utf16().count() <= MAX_TEXT);
    assert!(text.ends_with(&end()));
    let mut out = Text::default();
    out.line(&"😀".repeat(MAX_TEXT));
    assert!(out.truncated);
    assert!(out.units <= MAX_TEXT);
}
#[test]
fn semantic_identity_changes_only_after_committed_observation_or_audit() {
    let mut b = snapshot(vec![], vec![]);
    let original = b.clone();
    let event = |kind: &str, payload: Value| {
        serde_json::from_value(json!({"protocolVersion":"loom.live.v1","sessionId":"s","epoch":1,"sequence":1,"messageType":kind,"payload":payload})).unwrap()
    };
    b.apply(&[
        event("session_state", json!({})),
        event("input_event", json!({})),
    ])
    .unwrap();
    assert!(b.same_display(&original));
    assert!(b
        .apply(&[
            event("observation", observation(0)),
            event("trigger_event", json!({}))
        ])
        .is_err());
    assert!(b.same_display(&original));
    b.apply(&[event("observation", observation(0))]).unwrap();
    assert!(!b.same_display(&original));
}

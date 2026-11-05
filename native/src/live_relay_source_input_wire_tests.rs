//! 独立wire oracle：整批拒绝不产生部分输入，控制切换/取消与多轴鼠标/按键保持原能力边界。
use super::*;
use crate::live_relay_viewer_identity::{ControlCursor, Snapshot};
use serde_json::{json, Value};
fn events(rows: Vec<(&str, Value)>) -> Vec<ControlEvent> {
    let snapshot=Snapshot::parse(br#"{"session":{"protocolVersion":"loom.live.v1","sessionId":"live:test","sourceDeviceId":"source","sourceHookId":"hook","revision":1,"viewerDevices":[]},"epoch":1,"closed":false}"#,"live:test").unwrap();
    let cursor = ControlCursor::new(snapshot.identity().clone(), 0);
    let bytes=serde_json::to_vec(&json!({"protocolVersion":"loom.live.v1","next":rows.len(),"reset":false,"events":rows.into_iter().enumerate().map(|(i,(kind,payload))|json!({"protocolVersion":"loom.live.v1","sessionId":"live:test","epoch":1,"sequence":i+1,"messageType":kind,"payload":payload})).collect::<Vec<_>>()})).unwrap();
    // 独立构造envelope后经既有游标验证，再解析业务payload。
    cursor.prepare(&bytes).unwrap().events().iter().map(|e|serde_json::from_value(json!({"protocolVersion":"loom.live.v1","sessionId":"live:test","epoch":1,"sequence":e.sequence(),"messageType":e.kind(),"payload":serde_json::from_str::<Value>(e.payload()).unwrap()})).unwrap()).collect()
}
fn state(controller: Value) -> Value {
    json!({"revision":2,"visibility":"visible","viewers":["a","b"],"controllerDeviceId":controller})
}
fn input(actor: &str, sequence: u64, kind: Value) -> Value {
    json!({"inputSequence":sequence,"issuedAtMs":1,"sourceDeviceId":actor,"kind":kind})
}
#[test]
fn source_input_authority_switch_cancel_and_native_events_are_ordered() {
    let rows = events(vec![
        ("session_state", state(json!("a"))),
        (
            "input_event",
            input(
                "a",
                1,
                json!({"kind":"key","data":{"code":"vk:65","state":"pressed","modifiers":0}}),
            ),
        ),
        ("session_state", state(json!("b"))),
        (
            "input_event",
            input(
                "b",
                1,
                json!({"kind":"wheel","data":{"x":0.25,"y":0.5,"deltaX":120,"deltaY":0}}),
            ),
        ),
        ("input_event", input("b", 2, json!({"kind":"cancel"}))),
        (
            "input_event",
            input(
                "b",
                3,
                json!({"kind":"mouse_button","data":{"x":1.0,"y":0.0,"button":"left","state":"pressed","clickCount":2}}),
            ),
        ),
    ]);
    let (_, actions) = State::new(1).prepare(&rows).unwrap();
    assert_eq!(
        actions,
        vec![
            Action::Enable,
            Action::Input(Event::Key {
                code: 65,
                down: true
            }),
            Action::Release,
            Action::Enable,
            Action::Input(Event::Wheel {
                point: [0.25, 0.5],
                delta: 120,
                horizontal: true
            }),
            Action::Release,
            Action::Enable,
            Action::Input(Event::Button {
                point: [1.0, 0.0],
                button: Button::Left,
                down: true,
                double: true
            })
        ]
    );
}
#[test]
fn source_input_rejects_wrong_actor_replay_unknown_unsupported_and_partial_batch() {
    for bad in [
        input("b", 1, json!({"kind":"cancel"})),
        input("a", 1, json!({"kind":"cancel"})),
        input("a", 2, json!({"kind":"text","data":{"text":"no"}})),
        input(
            "a",
            2,
            json!({"kind":"key","data":{"code":"vk:65","state":"pressed","modifiers":1}}),
        ),
        input(
            "a",
            2,
            json!({"kind":"mouse_move","data":{"x":1.1,"y":0.0}}),
        ),
        input(
            "a",
            2,
            json!({"kind":"wheel","data":{"x":0.0,"y":0.0,"deltaX":120,"deltaY":120}}),
        ),
    ] {
        let rows = events(vec![
            ("session_state", state(json!("a"))),
            (
                "input_event",
                input(
                    "a",
                    1,
                    json!({"kind":"mouse_move","data":{"x":0.0,"y":0.0}}),
                ),
            ),
            ("input_event", bad),
        ]);
        assert!(State::new(1).prepare(&rows).is_err());
    }
    assert!(State::new(1)
        .prepare(&events(vec![("unknown", json!({}))]))
        .is_err());
    assert!(State::new(1)
        .prepare(&events(vec![(
            "input_event",
            input("a", 1, json!({"kind":"cancel"}))
        )]))
        .is_err());
}
#[test]
fn source_input_closed_or_regressed_state_cannot_restore_authority() {
    let mut closed = state(json!(null));
    closed["visibility"] = json!("closed");
    assert!(State::new(1)
        .prepare(&events(vec![
            ("session_state", closed),
            ("session_state", state(json!("a")))
        ]))
        .is_err());
    assert!(State::new(3)
        .prepare(&events(vec![("session_state", state(json!("a")))]))
        .is_err());
}
#[test]
fn source_loss_consumes_old_inputs_without_replay_until_reliable_release_and_new_grant() {
    let (mut source, _) = State::new(1)
        .prepare(&events(vec![("session_state", state(json!("a")))]))
        .unwrap();
    source.suspend();
    let key = |sequence| {
        (
            "input_event",
            input(
                "a",
                sequence,
                json!({"kind":"key","data":{"code":"vk:65","state":"pressed","modifiers":0}}),
            ),
        )
    };
    let (source, actions) = source.prepare(&events(vec![key(1), key(2)])).unwrap();
    assert!(actions.is_empty());
    assert!(source.prepare(&events(vec![key(2)])).is_err());
    let (_, actions) = source
        .prepare(&events(vec![
            ("session_state", state(json!(null))),
            ("session_state", state(json!("a"))),
            key(1),
        ]))
        .unwrap();
    assert_eq!(
        actions,
        vec![
            Action::Enable,
            Action::Input(Event::Key {
                code: 65,
                down: true
            })
        ]
    );
}

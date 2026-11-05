use super::*;
use crate::workspace_id::UnitId;
use std::rc::Rc;
struct Window(HWND);
impl Drop for Window {
    fn drop(&mut self) {
        unsafe {
            let _ = DestroyWindow(self.0);
        }
    }
}
fn selection() -> super::super::capability_selection::Selection {
    let (_listener, identity, _) = crate::loom_bridge_client::tests::fixture();
    let mut value = crate::loom_handshake::tests::fixture();
    value["capabilities"]["artDefinitions"][0]["parameters"]
        .as_array_mut()
        .unwrap()
        .push(serde_json::json!({"id":"amount","default":1}));
    let handshake = crate::loom_handshake::response(&serde_json::to_vec(&value).unwrap())
        .unwrap()
        .unwrap();
    let parameters = Parameters::new(&handshake.capability(0).unwrap()).unwrap();
    super::super::capability_selection::Selection {
        execution: Default::default(),
        identity: super::super::document_state::new_unit_id().unwrap(),
        parameters,
        catalog: Rc::new(super::super::capability_job::Loaded {
            identity,
            handshake,
        }),
    }
}
#[test]
fn parameter_commit_rejects_stale_dialogs_and_cancels_only_after_valid_change() {
    let state = State::new(None, false).unwrap();
    *state.selected_art.borrow_mut() = Some(selection());
    let initial = Edit::capture(&state).unwrap();
    assert_eq!(parse(&initial.text().unwrap()).unwrap()["amount"], 1);
    assert!(!initial.text().unwrap().contains("secret"));
    unsafe {
        let window = Window(
            CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                w!("STATIC"),
                w!("Parameter test"),
                WINDOW_STYLE::default(),
                0,
                0,
                1,
                1,
                None,
                None,
                None,
                None,
            )
            .unwrap(),
        );
        assert!(Edit::capture(&state)
            .unwrap()
            .commit(window.0, &state, r#"{"secret":"no"}"#)
            .is_err());
        assert!(state.execution_cancel.borrow().is_none());
        let request = crate::graph_requests::Request {
            id: UnitId::parse("old").unwrap(),
            generation: 1,
        };
        state.execution_results.borrow_mut().begin(request.clone());
        assert!(Edit::capture(&state)
            .unwrap()
            .commit(window.0, &state, r#"{"amount":2}"#)
            .unwrap());
        let scheduled = state.pending_start.borrow_mut().take().unwrap();
        assert_eq!(
            scheduled.trigger,
            crate::graph_execution_config::Trigger::Parameter { final_change: true }
        );
        assert!(scheduled.stamp.matches(&state));
        assert_eq!(
            state
                .execution_cancel
                .borrow_mut()
                .take()
                .unwrap()
                .execution,
            Some(request)
        );
        let configuration = crate::loom_art_wire::Configuration::from_parameters(
            &state.selected_art.borrow().as_ref().unwrap().parameters,
            &[],
        )
        .unwrap();
        let wire = crate::loom_art_wire::execute_config(
            &state.unit_id,
            &crate::graph_requests::Request {
                id: UnitId::parse("edited").unwrap(),
                generation: 2,
            },
            &configuration,
            &[],
            crate::loom_art_wire::OutputTransport::Websocket,
        )
        .unwrap();
        let wire: Value = serde_json::from_str(&wire).unwrap();
        assert_eq!(wire["params"]["parameters"]["amount"], 2);
        assert!(!initial.commit(window.0, &state, r#"{"amount":3}"#).unwrap());
        assert!(state.execution_cancel.borrow().is_none());
        let old_selection = Edit::capture(&state).unwrap();
        *state.selected_art.borrow_mut() = Some(selection());
        assert!(!old_selection
            .commit(window.0, &state, r#"{"amount":4}"#)
            .unwrap());
        state.group_locked.set(true);
        assert!(!Edit::capture(&state)
            .unwrap()
            .commit(window.0, &state, "{}")
            .unwrap());
        state.group_locked.set(false);
        assert!(!Edit::capture(&state)
            .unwrap()
            .commit(window.0, &state, r#"{"amount":1}"#)
            .unwrap());
        assert!(state.execution_cancel.borrow().is_none());
        assert!(Edit::capture(&state)
            .unwrap()
            .commit(window.0, &state, "{}")
            .unwrap());
        assert_eq!(
            state
                .selected_art
                .borrow()
                .as_ref()
                .unwrap()
                .parameters
                .get("amount"),
            Some(None)
        );
    }
}
#[test]
fn parameter_parser_requires_a_bounded_json_object() {
    for text in ["null", "[]", "true", "bad"] {
        assert!(parse(text).is_none());
    }
    assert!(parse(&" ".repeat(MAX_TEXT + 1)).is_none());
    assert!(parse(r#"{"value":null}"#).is_some());
}

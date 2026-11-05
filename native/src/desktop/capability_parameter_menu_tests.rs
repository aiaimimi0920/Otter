use super::*;
use std::rc::Rc;
fn state() -> State {
    let (_listener, identity, _) = crate::loom_bridge_client::tests::fixture();
    let mut value = crate::loom_handshake::tests::fixture();
    value["capabilities"]["artDefinitions"][0]["parameters"] = serde_json::json!([
        {"id":"enabled","type":"boolean","default":false},
        {"id":"mode","options":[{"value":"fast","label":"快速"}, {"value":2,"label":"数字"}],"default":"fast"},
        {"id":"locked","type":"bool","default":false,"disabled":true},
        {"id":"secret","type":"bool","default":true,"secret":true},
        {"id":"amount","type":"number","default":1},
        {"id":"tint","widget":"color","default":"#112233"},
        {"id":"payload","type":"json","default":{"answer":1}}
    ]);
    let handshake = crate::loom_handshake::response(&serde_json::to_vec(&value).unwrap())
        .unwrap()
        .unwrap();
    let parameters =
        crate::loom_parameter_state::Parameters::new(&handshake.capability(0).unwrap()).unwrap();
    let state = State::new(None, false).unwrap();
    *state.selected_art.borrow_mut() = Some(super::super::capability_selection::Selection {
        execution: Default::default(),
        identity: super::super::document_state::new_unit_id().unwrap(),
        parameters,
        catalog: Rc::new(super::super::capability_job::Loaded {
            identity,
            handshake,
        }),
    });
    state
}
struct Window(HWND);
impl Drop for Window {
    fn drop(&mut self) {
        unsafe {
            let _ = DestroyWindow(self.0);
        }
    }
}

#[test]
fn typed_menu_preserves_value_types_secret_boundary_and_stale_selection_guard() {
    let state = state();
    let snapshot = Snapshot::capture(&state).unwrap();
    assert_eq!(snapshot.choices.len(), 19);
    assert!(snapshot.choices.iter().all(|choice| choice.id != "secret"));
    assert!(matches!(snapshot.choices[4].action, Action::Edit(..)));
    assert!(matches!(snapshot.choices[5].action, Action::Color(..)));
    let Action::Edit(spec, text) = &snapshot.choices[7].action else {
        panic!("JSON editor missing");
    };
    assert_eq!(spec.parse(text), Some(serde_json::json!({"answer":1})));
    assert!(snapshot.choices[1].checked);
    unsafe {
        let window = Window(
            CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                w!("STATIC"),
                w!("Typed parameters"),
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
        let menu = Menu(CreatePopupMenu().unwrap());
        snapshot.append(menu.0, &state).unwrap();
        let child = GetSubMenu(menu.0, 0);
        assert_ne!(
            GetMenuState(child, (FIRST + 3) as u32, MF_BYCOMMAND) & MF_GRAYED.0,
            0
        );
        assert!(!snapshot.select(window.0, &state, FIRST + 3).unwrap());
        assert!(state.execution_cancel.borrow().is_none());
        assert!(snapshot.select(window.0, &state, FIRST).unwrap());
        assert_eq!(
            state
                .selected_art
                .borrow()
                .as_ref()
                .unwrap()
                .parameters
                .get("enabled"),
            Some(Some(&Value::Bool(true)))
        );
        assert!(state
            .pending_start
            .borrow()
            .as_ref()
            .unwrap()
            .stamp
            .matches(&state));
        assert!(!snapshot.select(window.0, &state, FIRST + 2).unwrap());
        state.execution_cancel.borrow_mut().take();
        let current = Snapshot::capture(&state).unwrap();
        assert!(current.select(window.0, &state, FIRST + 2).unwrap());
        assert_eq!(
            state
                .selected_art
                .borrow()
                .as_ref()
                .unwrap()
                .parameters
                .get("mode"),
            Some(Some(&serde_json::json!(2)))
        );
        assert_eq!(
            state
                .selected_art
                .borrow()
                .as_ref()
                .unwrap()
                .parameters
                .get("amount"),
            Some(Some(&serde_json::json!(1)))
        );
        assert!(state
            .selected_art
            .borrow()
            .as_ref()
            .unwrap()
            .parameters
            .get("secret")
            .is_none());
        let current = Snapshot::capture(&state).unwrap();
        assert!(!current.select(window.0, &state, FIRST + 2).unwrap());
        state.group_locked.set(true);
        assert!(!current.select(window.0, &state, FIRST).unwrap());
        state.group_locked.set(false);
        state
            .selected_art
            .borrow_mut()
            .as_mut()
            .unwrap()
            .parameters
            .set("tint", Some(Value::String("#ffffff".into())))
            .unwrap();
        assert!(Snapshot::capture(&state)
            .unwrap()
            .select(window.0, &state, FIRST + 6)
            .unwrap());
        assert_eq!(
            state
                .selected_art
                .borrow()
                .as_ref()
                .unwrap()
                .parameters
                .get("tint"),
            Some(Some(&Value::String(String::new())))
        );
        let snapshot = Snapshot::capture(&state).unwrap();
        let reset = snapshot
            .choices
            .iter()
            .position(|choice| choice.id == "enabled" && matches!(choice.action, Action::Reset(_)))
            .unwrap();
        assert!(snapshot.select(window.0, &state, FIRST + reset).unwrap());
        assert_eq!(
            state
                .selected_art
                .borrow()
                .as_ref()
                .unwrap()
                .parameters
                .get("enabled"),
            Some(Some(&Value::Bool(false)))
        );
        for expected in [
            Value::String(crate::graph_params::DISABLED.into()),
            Value::from(1),
        ] {
            let snapshot = Snapshot::capture(&state).unwrap();
            let toggle = snapshot
                .choices
                .iter()
                .position(|choice| {
                    choice.id == "amount" && matches!(choice.action, Action::ToggleDisabled(_))
                })
                .unwrap();
            assert!(snapshot.select(window.0, &state, FIRST + toggle).unwrap());
            assert_eq!(
                state
                    .selected_art
                    .borrow()
                    .as_ref()
                    .unwrap()
                    .parameters
                    .get("amount"),
                Some(Some(&expected))
            );
            if expected.as_str() == Some(crate::graph_params::DISABLED) {
                let snapshot = Snapshot::capture(&state).unwrap();
                assert!(snapshot
                    .choices
                    .iter()
                    .filter(|choice| choice.id == "amount"
                        && !matches!(choice.action, Action::ToggleDisabled(_)))
                    .all(|choice| !choice.enabled));
                let selected = state.selected_art.borrow();
                let configuration = crate::loom_art_wire::Configuration::from_parameters(
                    &selected.as_ref().unwrap().parameters,
                    &[],
                )
                .unwrap();
                let wire = crate::loom_art_wire::execute_config(
                    &state.unit_id,
                    &crate::graph_requests::Request {
                        id: crate::workspace_id::UnitId::parse("disabled-test").unwrap(),
                        generation: 1,
                    },
                    &configuration,
                    &[],
                    crate::loom_art_wire::OutputTransport::Websocket,
                )
                .unwrap();
                let wire: Value = serde_json::from_str(&wire).unwrap();
                assert!(wire["params"]["parameters"].get("amount").is_none());
                assert!(wire["params"]["disabledParameters"]
                    .as_array()
                    .unwrap()
                    .contains(&Value::String("amount".into())));
            }
        }
    }
}

#[test]
fn reset_contract_preserves_unset_null_and_widget_fallbacks() {
    fn reset(parameter: Value) -> Option<Option<Value>> {
        let raw = serde_json::json!({"id":"art","parameters":[parameter]});
        let capability = crate::loom_capability::Capability::new(&raw).unwrap();
        let result = reset_value(&capability.parameters().next().unwrap());
        result
    }
    assert_eq!(
        reset(serde_json::json!({"id":"mode","widget":"select"})),
        Some(None)
    );
    assert_eq!(
        reset(serde_json::json!({"id":"mode","widget":"radio","default":null})),
        Some(Some(Value::Null))
    );
    for (widget, expected) in [
        ("number", serde_json::json!(0)),
        ("switch", serde_json::json!(false)),
        ("path", serde_json::json!("")),
        ("image_link", serde_json::json!("")),
    ] {
        assert_eq!(
            reset(serde_json::json!({"id":"value","widget":widget,"default":null})),
            Some(Some(expected))
        );
    }
    assert_eq!(
        reset(
            serde_json::json!({"id":"token","widget":"text","secret":true,"default":"must-not-leak"})
        ),
        None
    );
    let state = state();
    unsafe {
        assert_eq!(
            Action::Reset(None).choose(HWND::default(), &state),
            Some(None)
        );
        assert_eq!(
            Action::Reset(Some(Value::Null)).choose(HWND::default(), &state),
            Some(Some(Value::Null))
        );
    }
}

#[test]
fn linked_view_locks_editing_rejects_stale_menu_and_preserves_manual_values() {
    let state = state();
    let values = serde_json::json!({"amount":7}).as_object().unwrap().clone();
    super::super::parameter_view::View::update(&state, values.clone(), vec!["amount".into()]);
    let identity = state
        .parameter_view
        .borrow()
        .as_ref()
        .unwrap()
        .identity
        .clone();
    super::super::parameter_view::View::update(&state, values, vec!["amount".into()]);
    assert!(std::sync::Arc::ptr_eq(
        &identity,
        &state.parameter_view.borrow().as_ref().unwrap().identity
    ));
    let old = Snapshot::capture(&state).unwrap();
    let edit = old
        .choices
        .iter()
        .find(|choice| choice.id == "amount" && matches!(choice.action, Action::Edit(..)))
        .unwrap();
    let Action::Edit(_, text) = &edit.action else {
        unreachable!();
    };
    assert_eq!(text, "7");
    assert!(edit.label.contains("7（连线）"));
    assert!(!edit.enabled);
    assert_eq!(
        state
            .selected_art
            .borrow()
            .as_ref()
            .unwrap()
            .parameters
            .get("amount"),
        Some(Some(&Value::from(1)))
    );
    super::super::parameter_view::View::update(
        &state,
        serde_json::json!({"amount":8}).as_object().unwrap().clone(),
        vec!["amount".into()],
    );
    unsafe {
        let window = Window(
            CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                w!("STATIC"),
                None,
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
        assert!(!old.select(window.0, &state, FIRST).unwrap());
        assert!(state.execution_cancel.borrow().is_none());
    }
    super::super::parameter_view::View::update(&state, Default::default(), Vec::new());
    let current = Snapshot::capture(&state).unwrap();
    let edit = current
        .choices
        .iter()
        .find(|choice| choice.id == "amount" && matches!(choice.action, Action::Edit(..)))
        .unwrap();
    let Action::Edit(_, text) = &edit.action else {
        unreachable!();
    };
    assert_eq!(text, "1");
    assert!(edit.enabled);
}

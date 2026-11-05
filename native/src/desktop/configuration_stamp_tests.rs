use super::*;
use crate::{loom_capability::Capability, loom_parameter_state::Parameters};
use serde_json::json;
use std::{rc::Rc, time::Duration};
fn select(state: &State) {
    let (_listener, identity, _) = crate::loom_bridge_client::tests::fixture();
    let handshake = crate::loom_handshake::response(
        &serde_json::to_vec(&crate::loom_handshake::tests::fixture()).unwrap(),
    )
    .unwrap()
    .unwrap();
    let raw = json!({"id":"test-art","parameters":[{"id":"amount","default":1}]});
    *state.selected_art.borrow_mut() = Some(super::super::capability_selection::Selection {
        execution: Default::default(),
        identity: super::super::document_state::new_unit_id().unwrap(),
        catalog: Rc::new(super::super::capability_job::Loaded {
            identity,
            handshake,
        }),
        parameters: Parameters::new(&Capability::new(&raw).unwrap()).unwrap(),
    });
}
#[test]
fn stamp_binds_document_selection_and_revision_but_not_unchanged_edits() {
    let state = State::new(None, false).unwrap();
    let empty = Stamp::capture(&state);
    assert!(empty.matches(&state));
    assert!(!empty.matches(&State::new(None, false).unwrap()));
    select(&state);
    assert!(!empty.matches(&state));
    let selected = Stamp::capture(&state);
    assert!(selected.matches(&state));
    state
        .selected_art
        .borrow_mut()
        .as_mut()
        .unwrap()
        .parameters
        .set("amount", Some(json!(1)))
        .unwrap();
    assert!(selected.matches(&state));
    state
        .selected_art
        .borrow_mut()
        .as_mut()
        .unwrap()
        .parameters
        .set("amount", Some(json!(2)))
        .unwrap();
    assert!(!selected.matches(&state));
    let edited = Stamp::capture(&state);
    select(&state);
    assert!(!edited.matches(&state));
    let reselected = Stamp::capture(&state);
    state.selected_art.borrow_mut().take();
    assert!(!reselected.matches(&state));
}
#[test]
fn stale_pending_configuration_is_discarded_before_request_registration() {
    stale_pending(false);
}
#[test]
fn changed_topology_discards_preparation_even_when_parameters_are_unchanged() {
    stale_pending(true);
}
fn stale_pending(topology: bool) {
    let mut workspace = crate::workspace::Workspace::default();
    let state = State::new(None, false).unwrap();
    select(&state);
    let (listener, identity, _) = crate::loom_bridge_client::tests::fixture();
    listener.set_nonblocking(true).unwrap();
    let configuration = crate::loom_art_wire::Configuration::from_parameters(
        &state.selected_art.borrow().as_ref().unwrap().parameters,
        &[],
    )
    .unwrap();
    *state.pending_preparation.borrow_mut() = Some(super::super::loom_execution_prepare::Pending {
        shader: None,
        dependencies: Vec::new(),
        ticket: UnitId::parse("ticket").unwrap(),
        topology: workspace.topology_stamp(),
        selection: Stamp::capture(&state),
        plan: super::super::loom_execution_prepare::Configured {
            identity,
            request_id: UnitId::parse("request").unwrap(),
            configuration,
            timeout: Duration::from_secs(5),
            transport: crate::loom_art_wire::OutputTransport::Websocket,
        },
    });
    discard_changed(&state, &workspace);
    assert!(state.pending_preparation.borrow().is_some());
    assert!(state.preparation_error.borrow().is_none());
    if topology {
        workspace
            .add_units([UnitId::parse("new-node").unwrap()])
            .unwrap();
    } else {
        state
            .selected_art
            .borrow_mut()
            .as_mut()
            .unwrap()
            .parameters
            .set("amount", Some(json!(3)))
            .unwrap();
    }
    // 不依赖 UI 取消消息；配置凭证本身足以退休旧准备。
    discard_changed(&state, &workspace);
    assert!(state.pending_preparation.borrow().is_none());
    assert!(state.preparation_error.borrow().is_some());
    assert!(state.pending_execution.borrow().is_none());
    assert!(state.execution_cancel.borrow().is_none());
    assert_eq!(
        listener.accept().unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );
}

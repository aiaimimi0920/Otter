use super::*;
use crate::workspace_id::UnitId;
#[test]
fn topology_change_retires_matching_execution_without_parameter_change() {
    let mut workspace = crate::workspace::Workspace::default();
    let state = State::new(None, false).unwrap();
    let mut requests = Requests::default();
    let request = requests
        .begin(state.unit_id.clone(), UnitId::parse("request").unwrap())
        .unwrap();
    state.execution_results.borrow_mut().begin(request.clone());
    *state.execution_configuration.borrow_mut() = Some(Binding {
        request: request.clone(),
        stamp: Stamp::capture(&state),
        topology: workspace.topology_stamp(),
    });
    assert!(matches(&state, &request, &workspace));
    workspace
        .add_units([UnitId::parse("new-node").unwrap()])
        .unwrap();
    assert!(!matches(&state, &request, &workspace));
    retire_changed(&state, &mut requests, &workspace);
    assert!(requests.active(&state.unit_id).is_none());
    assert!(state.execution_results.borrow().active().is_none());
}
#[test]
fn changed_configuration_ends_only_its_registered_request() {
    let workspace = crate::workspace::Workspace::default();
    let state = State::new(None, false).unwrap();
    let other = State::new(None, false).unwrap();
    let mut requests = Requests::default();
    let request = requests
        .begin(state.unit_id.clone(), UnitId::parse("old").unwrap())
        .unwrap();
    state.execution_results.borrow_mut().begin(request.clone());
    *state.execution_configuration.borrow_mut() = Some(Binding {
        request: request.clone(),
        stamp: Stamp::capture(&state),
        topology: workspace.topology_stamp(),
    });
    assert!(matches(&state, &request, &workspace));
    retire_changed(&state, &mut requests, &workspace);
    assert!(requests.is_latest(&state.unit_id, &request));
    state
        .execution_configuration
        .borrow_mut()
        .as_mut()
        .unwrap()
        .stamp = Stamp::capture(&other);
    assert!(!matches(&state, &request, &workspace));
    retire_changed(&state, &mut requests, &workspace);
    assert!(requests.active(&state.unit_id).is_none());
    assert!(state.execution_configuration.borrow().is_none());
    assert!(state.execution_results.borrow().active().is_none());
}
#[test]
fn stale_binding_cannot_cancel_new_generation_or_clear_new_binding() {
    let workspace = crate::workspace::Workspace::default();
    let state = State::new(None, false).unwrap();
    let other = State::new(None, false).unwrap();
    let mut requests = Requests::default();
    let old = requests
        .begin(state.unit_id.clone(), UnitId::parse("same").unwrap())
        .unwrap();
    let new = requests
        .begin(state.unit_id.clone(), UnitId::parse("same").unwrap())
        .unwrap();
    state.execution_results.borrow_mut().begin(new.clone());
    *state.execution_configuration.borrow_mut() = Some(Binding {
        request: old.clone(),
        stamp: Stamp::capture(&other),
        topology: workspace.topology_stamp(),
    });
    retire_changed(&state, &mut requests, &workspace);
    assert!(requests.is_latest(&state.unit_id, &new));
    assert_eq!(state.execution_results.borrow().active(), Some(&new));
    *state.execution_configuration.borrow_mut() = Some(Binding {
        request: new.clone(),
        stamp: Stamp::capture(&state),
        topology: workspace.topology_stamp(),
    });
    clear(&state, &old);
    assert!(matches(&state, &new, &workspace));
    clear(&state, &new);
    assert!(state.execution_configuration.borrow().is_none());
}

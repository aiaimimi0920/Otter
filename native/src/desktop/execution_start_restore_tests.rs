//! 恢复预览保持的真实命令入口回归；隐藏窗口，不连接真实 Loom。
use super::*;
use crate::desktop::{
    capability_job::Loaded, capability_selection::Selection, image::Image,
    loom_result_state::Results,
};
use std::rc::Rc;

fn state() -> State {
    let state = State::new(None, false).unwrap();
    let (_listener, identity, _) = crate::loom_bridge_client::tests::fixture();
    let raw = crate::loom_handshake::tests::fixture();
    let catalog = Rc::new(Loaded {
        identity,
        handshake: crate::loom_handshake::response(&serde_json::to_vec(&raw).unwrap())
            .unwrap()
            .unwrap(),
    });
    let capability = catalog.handshake.capability(0).unwrap();
    let mut execution = capability.execution_config(None);
    execution.param_driven = false;
    *state.selected_art.borrow_mut() = Some(Selection {
        execution,
        identity: crate::desktop::document_state::new_unit_id().unwrap(),
        parameters: crate::loom_parameter_state::Parameters::new(&capability).unwrap(),
        catalog: catalog.clone(),
    });
    restore(&state);
    state
}
fn restore(state: &State) {
    *state.execution_results.borrow_mut() = Results::from_restored_display(
        state.unit_id.clone(),
        Some(Image {
            width: 1,
            height: 1,
            pixels: vec![255; 4].into(),
        }),
        true,
    );
}
fn assert_unlocked(state: &State) {
    let results = state.execution_results.borrow();
    assert!(!results.holds_restored_preview());
    assert!(results.display().is_some());
    assert!(results.ports().is_empty() && results.active().is_none());
}
#[test]
fn real_parameter_and_upstream_changes_unlock_independently_of_execution_gate() {
    let state = state();
    state.group_locked.set(true);
    parameter_changed(&state);
    assert!(state.execution_results.borrow().holds_restored_preview());
    state.group_locked.set(false);
    parameter_changed(&state);
    assert_unlocked(&state);
    assert!(
        !state
            .selected_art
            .borrow()
            .as_ref()
            .unwrap()
            .execution
            .param_driven
    );
    restore(&state);
    upstream_changed(&state);
    assert_unlocked(&state);
}
#[test]
fn accepted_manual_command_unlocks_but_invalid_window_does_not() {
    let state = state();
    unsafe {
        assert!(request(HWND::default(), &state).is_err());
        assert!(state.execution_results.borrow().holds_restored_preview());
        let hwnd = CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            w!("STATIC"),
            w!("Restore preview test"),
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
        .unwrap();
        struct Window(HWND);
        impl Drop for Window {
            fn drop(&mut self) {
                unsafe {
                    let _ = DestroyWindow(self.0);
                }
            }
        }
        let window = Window(hwnd);
        request(window.0, &state).unwrap();
        assert_unlocked(&state);
        assert!(matches!(
            state.pending_start.borrow().as_ref().unwrap().trigger,
            crate::graph_execution_config::Trigger::Manual
        ));
    }
}

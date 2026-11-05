use super::*;
use std::rc::Rc;
struct Window(HWND);
impl Drop for Window {
    fn drop(&mut self) {
        unsafe {
            let _ = DestroyWindow(self.0);
        }
    }
}
struct Menu(HMENU);
impl Drop for Menu {
    fn drop(&mut self) {
        unsafe {
            let _ = DestroyMenu(self.0);
        }
    }
}
#[test]
fn policy_controls_update_node_stamp_without_scheduling_and_obey_lock() {
    let state = State::new(None, false).unwrap();
    let (_, identity, _) = crate::loom_bridge_client::tests::fixture();
    let handshake = crate::loom_handshake::response(
        &serde_json::to_vec(&crate::loom_handshake::tests::fixture()).unwrap(),
    )
    .unwrap()
    .unwrap();
    let parameters =
        crate::loom_parameter_state::Parameters::new(&handshake.capability(0).unwrap()).unwrap();
    *state.selected_art.borrow_mut() = Some(crate::desktop::capability_selection::Selection {
        execution: Config::default(),
        identity: crate::desktop::document_state::new_unit_id().unwrap(),
        parameters,
        catalog: Rc::new(crate::desktop::capability_job::Loaded {
            identity,
            handshake,
        }),
    });
    unsafe {
        let window = Window(
            CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                w!("STATIC"),
                w!("Execution settings test"),
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
        let initial = crate::desktop::configuration_stamp::Stamp::capture(&state);
        state.group_locked.set(true);
        toggle(window.0, &state, FIRST).unwrap();
        assert!(initial.matches(&state));
        state.group_locked.set(false);
        let old = crate::graph_requests::Request {
            id: crate::workspace_id::UnitId::parse("old").unwrap(),
            generation: 1,
        };
        state.execution_results.borrow_mut().begin(old.clone());
        for index in 0..4 {
            let before = crate::desktop::configuration_stamp::Stamp::capture(&state);
            crate::desktop::context_menu::execute(window.0, &state, FIRST + index).unwrap();
            assert!(!before.matches(&state));
            assert!(!flags(state.selected_art.borrow().as_ref().unwrap().execution)[index]);
            assert!(state.pending_start.borrow().is_none());
        }
        assert_eq!(
            state.execution_cancel.borrow().as_ref().unwrap().execution,
            Some(old)
        );
        let config = state.selected_art.borrow().as_ref().unwrap().execution;
        assert!(!config.accepts_upstream_notification());
        assert_eq!(
            config.gate(
                crate::graph_execution_config::Trigger::Manual,
                Default::default()
            ),
            Ok(())
        );
        let menu = Menu(CreatePopupMenu().unwrap());
        append(menu.0, &state).unwrap();
        for index in 0..4 {
            assert_eq!(
                GetMenuState(menu.0, (FIRST + index) as u32, MF_BYCOMMAND) & MF_CHECKED.0,
                0
            );
        }
        let before = crate::desktop::configuration_stamp::Stamp::capture(&state);
        toggle(HWND::default(), &state, FIRST).unwrap();
        assert!(before.matches(&state));
    }
}

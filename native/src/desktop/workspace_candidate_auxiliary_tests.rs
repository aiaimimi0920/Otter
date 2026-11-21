//! Application 到真实右 strip/候选 child 的闭环，复用原候选提交与配置凭证。
use super::*;
use serde_json::json;
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::UI::Input::KeyboardAndMouse::{GetFocus, IsWindowEnabled, SetFocus, VK_ESCAPE};
unsafe fn app(count: usize, empty: bool) -> Application {
    let mut app = super::super::tests::app();
    let (_listener, identity, _) = crate::loom_bridge_client::tests::fixture();
    let mut value = crate::loom_handshake::tests::fixture();
    value["capabilities"]["artDefinitions"][0]["parameters"] = if empty {
        json!([])
    } else {
        json!([
            {"id":"result_index","type":"number","default":0,"min":0,"max":63,"step":1},
            {"id":"query","type":"string","default":"原查询"}
        ])
    };
    value["capabilities"]["artDefinitions"][0]["inputs"] = json!([]);
    value["capabilities"]["artDefinitions"][0]["outputs"] = json!([]);
    let handshake = crate::loom_handshake::response(&serde_json::to_vec(&value).unwrap())
        .unwrap()
        .unwrap();
    *app.documents[0].state.selected_art.borrow_mut() =
        Some(desktop::capability_selection::Selection {
            execution: Default::default(),
            identity: desktop::document_state::new_unit_id().unwrap(),
            parameters: crate::loom_parameter_state::Parameters::new(
                &handshake.capability(0).unwrap(),
            )
            .unwrap(),
            catalog: Rc::new(desktop::capability_job::Loaded {
                identity,
                handshake,
            }),
        });
    SetWindowPos(
        app.documents[0].window.0,
        None,
        400,
        120,
        240,
        180,
        SWP_NOACTIVATE | SWP_NOZORDER,
    )
    .unwrap();
    let _ = ShowWindow(app.documents[0].window.0, SW_SHOWNOACTIVATE);
    app.sync_link_menus();
    let metadata = json!({"items":(0..count).map(|i|json!({"index":i,"title":format!("候选 {i}")})).collect::<Vec<_>>(),"selectedIndex":0});
    let batch = desktop::loom_output_batch::decode(
        &json!({"output":{"kind":"value","value":42}}),
        Some(&metadata),
        &std::sync::atomic::AtomicBool::new(false),
    )
    .unwrap();
    let state = &app.documents[0].state;
    let request = crate::graph_requests::Request {
        id: crate::workspace_id::UnitId::parse("candidate-result").unwrap(),
        generation: 1,
    };
    let mut results = state.execution_results.borrow_mut();
    results.begin(request.clone());
    results.finish(&request, batch).unwrap();
    results.receive_candidates(Some(metadata));
    results.bind_candidate_configuration(desktop::configuration_stamp::Stamp::capture(state));
    drop(results);
    state.parameters_ui.toggle.set(true);
    app.process_parameter_panels(true);
    app.sync_workspace_chrome(true);
    assert!(app.chrome.ui.failure.borrow().is_none());
    app
}
unsafe fn strip(app: &Application) -> HWND {
    app.chrome
        .ui
        .input_windows
        .borrow()
        .iter()
        .find(|(_, n, s)| *n == 0 && *s == Side::Output)
        .unwrap()
        .0
}
unsafe fn grid(app: &Application) -> HWND {
    let aux = FindWindowExW(
        Some(strip(app)),
        None,
        w!("Otter.Native.CandidateAuxiliary"),
        None,
    )
    .unwrap();
    GetWindow(aux, GW_CHILD).unwrap()
}
unsafe fn notifications(aux: HWND) {
    let mut msg = MSG::default();
    while PeekMessageW(&mut msg, Some(aux), WM_APP + 96, WM_APP + 96, PM_REMOVE).as_bool() {
        DispatchMessageW(&msg);
    }
}
#[test]
fn total_tab_attaches_candidate_grid_after_outputs_with_actual_parent_region_and_shared_transaction(
) {
    unsafe {
        let mut app = app(2, false);
        let grid = grid(&app);
        let aux = GetParent(grid).unwrap();
        let root = strip(&app);
        assert_eq!(GetAncestor(grid, GA_ROOT), root);
        assert!(GetWindowLongW(grid, GWL_STYLE) as u32 & WS_CHILD.0 != 0);
        let expected = layout(&app.chrome.ui, &app.chrome.ui.node(0).unwrap()).unwrap();
        let mut actual = RECT::default();
        GetWindowRect(aux, &mut actual).unwrap();
        assert_eq!(actual, expected);
        let mut origin = RECT::default();
        GetWindowRect(root, &mut origin).unwrap();
        let region = CreateRectRgn(0, 0, 0, 0);
        assert_ne!(GetWindowRgn(root, region), RGN_ERROR);
        assert!(PtInRegion(
            region,
            actual.right - origin.left - 4,
            actual.top - origin.top + 4
        )
        .as_bool());
        let _ = DeleteObject(HGDIOBJ(region.0));
        let button = GetDlgItem(Some(grid), 2).unwrap();
        SendMessageW(button, BM_CLICK, None, None);
        notifications(aux);
        assert!(!app.documents[0].parameters.as_ref().unwrap().exit_idle());
        app.process_parameter_panels(true);
        assert_eq!(
            app.documents[0]
                .state
                .selected_art
                .borrow()
                .as_ref()
                .unwrap()
                .parameters
                .get("result_index")
                .flatten(),
            Some(&json!(1))
        );
        assert_eq!(
            app.documents[0]
                .state
                .pending_start
                .borrow()
                .as_ref()
                .unwrap()
                .trigger,
            crate::graph_execution_config::Trigger::Manual
        );
        app.shutdown();
        assert!(!IsWindow(Some(grid)).as_bool());
    }
}
#[test]
fn candidates_without_public_parameters_or_outputs_still_have_one_document_owner_and_disabled_cards(
) {
    unsafe {
        let mut app = app(2, true);
        assert_eq!(app.chrome.ui.rails.borrow().len(), 1);
        let grid = grid(&app);
        assert!(!IsWindowEnabled(GetDlgItem(Some(grid), 1).unwrap()).as_bool());
        assert!(app.documents[0].parameters.is_some());
        app.shutdown();
        assert!(!IsWindow(Some(grid)).as_bool());
    }
}
#[test]
fn collapsed_tab_and_configuration_change_retire_queued_candidate_clicks_without_execution() {
    unsafe {
        for change_config in [false, true] {
            let mut app = app(2, false);
            let grid = grid(&app);
            let aux = GetParent(grid).unwrap();
            SendMessageW(GetDlgItem(Some(grid), 2).unwrap(), BM_CLICK, None, None);
            if change_config {
                app.documents[0]
                    .state
                    .selected_art
                    .borrow_mut()
                    .as_mut()
                    .unwrap()
                    .parameters
                    .set("query", Some(json!("new")))
                    .unwrap();
            } else {
                app.documents[0].state.parameters_ui.toggle.set(true);
                app.process_parameter_panels(true);
                app.sync_workspace_chrome(true);
            }
            notifications(aux);
            app.process_parameter_panels(true);
            assert!(app.documents[0].state.pending_start.borrow().is_none());
            assert_eq!(
                app.documents[0]
                    .state
                    .selected_art
                    .borrow()
                    .as_ref()
                    .unwrap()
                    .parameters
                    .get("result_index")
                    .flatten(),
                Some(&json!(0))
            );
            app.shutdown();
        }
    }
}
#[test]
fn right_grid_scroll_is_local_and_moving_the_pin_moves_the_same_child_tree() {
    unsafe {
        let mut app = app(20, false);
        let grid = grid(&app);
        let button = GetDlgItem(Some(grid), 20).unwrap();
        let left = app.chrome.ui.rail_offsets.borrow().clone();
        let mut before = RECT::default();
        GetWindowRect(button, &mut before).unwrap();
        SendMessageW(
            grid,
            WM_MOUSEWHEEL,
            Some(WPARAM(((-120i16) as u16 as usize) << 16)),
            None,
        );
        let mut scrolled = RECT::default();
        GetWindowRect(button, &mut scrolled).unwrap();
        assert_eq!(scrolled.top, before.top - 48);
        assert_eq!(*app.chrome.ui.rail_offsets.borrow(), left);
        assert_eq!(GetDlgItem(Some(grid), 20).unwrap(), button);
        let host = app.documents[0].window.0;
        let mut pin = RECT::default();
        GetWindowRect(host, &mut pin).unwrap();
        SetWindowPos(
            host,
            None,
            pin.left + 30,
            pin.top + 20,
            0,
            0,
            SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
        )
        .unwrap();
        app.sync_workspace_chrome(true);
        let mut moved = RECT::default();
        GetWindowRect(button, &mut moved).unwrap();
        assert_eq!(
            (moved.left - scrolled.left, moved.top - scrolled.top),
            (30, 20)
        );
        assert_eq!(self::grid(&app), grid);
        app.shutdown();
    }
}
#[test]
fn escape_from_candidate_button_closes_parameters_and_restores_pin_focus_without_deletion() {
    unsafe {
        let mut app = app(2, false);
        let button = GetDlgItem(Some(grid(&app)), 1).unwrap();
        let host = app.documents[0].window.0;
        let _ = SetFocus(Some(button));
        assert_eq!(GetFocus(), button);
        assert!(app.documents[0]
            .parameters
            .as_ref()
            .unwrap()
            .translate(&MSG {
                hwnd: button,
                message: WM_KEYDOWN,
                wParam: WPARAM(VK_ESCAPE.0 as usize),
                ..Default::default()
            }));
        assert_eq!(GetFocus(), host);
        assert!(IsWindow(Some(host)).as_bool());
        assert!(!app.documents[0].state.parameters_ui.expanded.get());
        app.shutdown();
    }
}

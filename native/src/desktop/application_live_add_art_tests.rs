//! 隐藏 Live/WARP 的 Add Art、实际模态控件与真实输入执行；不冒充普通前台验收。
use super::*;
use crate::desktop::node_menu::art_picker::{dialog, tests as fixture};
use windows::Win32::UI::Input::KeyboardAndMouse::*;
#[path = "application_live_art_load_runtime_tests.rs"]
mod load;
unsafe fn configured(app: &Application) {
    app.catalog.borrow_mut().loaded = Some(fixture::catalog(|_| {}));
}
unsafe fn add(app: &mut Application, unit: &crate::workspace_id::UnitId) -> Result<bool> {
    app.create_live_art_with(unit, |_, _, _| Ok(dialog::Outcome::Select(0)))
}
#[test]
fn live_add_art_actual_owned_picker_search_ime_affinity_and_independent_child() {
    unsafe {
        let environment = Environment::new();
        let mut app = environment.app();
        configured(&app);
        let (hwnd, unit) = live(&mut app, true);
        let frame = app.live_sessions[0].snapshot_frame().unwrap().unwrap();
        assert!(app
            .create_live_art_with(&unit, |app, owner, picker| {
                assert!(app.modals.blocked());
                assert_eq!(owner, hwnd);
                dialog::tests::live_check(owner);
                let result = picker.show(app, owner);
                dialog::tests::check_hook();
                result
            })
            .unwrap());
        assert_eq!(app.documents.len(), 2);
        assert_eq!(app.live_sessions.len(), 1);
        assert!(app.live_sessions[0].is_alive());
        assert!(Arc::ptr_eq(
            &frame,
            &app.live_sessions[0].snapshot_frame().unwrap().unwrap()
        ));
        let child = &app.documents[1].state;
        assert_ne!(child.unit_id, unit);
        let edge = &app.workspace.links()[0];
        assert_eq!(edge.from_unit, unit);
        assert_eq!(edge.to_unit, child.unit_id);
        assert_eq!((&*edge.from_port, &*edge.to_port), ("output", "pixels"));
        assert_eq!(
            child.pending_start.borrow().as_ref().unwrap().trigger,
            crate::graph_execution_config::Trigger::Upstream
        );
        assert_eq!(
            child
                .selected_art
                .borrow()
                .as_ref()
                .unwrap()
                .parameters
                .get("amount"),
            Some(Some(&serde_json::json!(42)))
        );
        assert!(!app.modals.blocked());
        app.shutdown();
    }
}
#[test]
fn live_add_art_cancel_invalid_disabled_and_dialog_error_preserve_source_and_graph() {
    unsafe {
        let environment = Environment::new();
        for mode in 0..4 {
            let mut app = environment.app();
            configured(&app);
            if mode == 2 {
                app.catalog.borrow_mut().loaded = Some(fixture::catalog(|raw| {
                    raw["capabilities"]["artDefinitions"][0]["enabled"] = false.into();
                }));
            }
            let (_, unit) = live(&mut app, true);
            let topology = app.workspace.topology_stamp();
            let result = app.create_live_art_with(&unit, |_, _, _| match mode {
                0 => Ok(dialog::Outcome::Cancel),
                1 => Ok(dialog::Outcome::Select(usize::MAX)),
                2 => Ok(dialog::Outcome::Select(0)),
                _ => Err(Error::new(E_FAIL, "injected picker error")),
            });
            if mode == 0 {
                assert!(!result.unwrap());
            } else {
                assert!(result.is_err());
            }
            assert!(topology.matches(&app.workspace));
            assert_eq!(app.documents.len(), 1);
            assert!(app.live_sessions[0].is_alive());
            assert!(!app.modals.blocked());
            app.shutdown();
        }
    }
}
#[test]
fn live_add_art_rechecks_catalog_source_group_workspace_and_session_after_modal() {
    unsafe {
        let environment = Environment::new();
        for mode in 0..6 {
            let mut app = environment.app();
            configured(&app);
            let (hwnd, unit) = live(&mut app, true);
            assert!(app
                .create_live_art_with(&unit, |app, _, _| {
                    match mode {
                        0 => configured(app),
                        1 => {
                            DestroyWindow(hwnd).unwrap();
                        }
                        2 => {
                            app.workspace = crate::workspace::Workspace::with_initial(
                                app.documents[0].state.unit_id.clone(),
                            )
                        }
                        3 => {
                            let group =
                                crate::workspace_id::UnitId::parse("live-art-locked").unwrap();
                            app.workspace
                                .upsert_group(crate::workspace::Group {
                                    id: group.clone(),
                                    name: "锁定".into(),
                                    hidden: false,
                                    locked: true,
                                })
                                .unwrap();
                            app.workspace
                                .assign_group(std::slice::from_ref(&unit), Some(&group))
                                .unwrap();
                        }
                        4 => {
                            app.authorization.borrow_mut().origin =
                                Some("https://changed.invalid".into())
                        }
                        _ => app.exit_requested.set(true),
                    }
                    Ok(dialog::Outcome::Select(0))
                })
                .is_err());
            assert_eq!(app.documents.len(), 1);
            assert!(app.workspace.links().is_empty());
            assert!(!app.modals.blocked());
            app.shutdown();
        }
    }
}
#[test]
fn live_add_art_refresh_defers_catalog_job_and_busy_session_does_not_open_picker() {
    unsafe {
        let environment = Environment::new();
        let mut app = environment.app();
        let (hwnd, unit) = live(&mut app, true);
        assert!(!app
            .create_live_art_with(&unit, |_, _, _| Ok(dialog::Outcome::Refresh))
            .unwrap());
        assert!(app.catalog.borrow().requested && !app.catalog.borrow().job.is_busy());
        app.catalog.borrow_mut().requested = false;
        crate::desktop::session_end::with_query(
            &app.session_end,
            || true,
            || {
                crate::desktop::session_end::with_owner(&app.session_end, || {
                    assert_eq!(
                        SendMessageW(hwnd, WM_QUERYENDSESSION, None, None),
                        LRESULT(1)
                    );
                });
            },
        );
        assert!(app.session_end.paused());
        assert!(app
            .create_live_art_with(&unit, |_, _, _| panic!("paused must not open picker"))
            .is_err());
        crate::desktop::session_end::with_owner(&app.session_end, || {
            SendMessageW(hwnd, WM_ENDSESSION, Some(WPARAM(0)), None);
        });
        assert!(!app.session_end.paused());
        app.shutdown();
    }
}
struct Keys([u8; 256]);
impl Keys {
    unsafe fn new() -> Self {
        let mut before = [0; 256];
        GetKeyboardState(&mut before).unwrap();
        Self(before)
    }
    unsafe fn set(&self, held: &[VIRTUAL_KEY]) {
        let mut keys = [0; 256];
        for key in held {
            keys[key.0 as usize] = 0x80;
        }
        SetKeyboardState(&keys).unwrap();
    }
}
impl Drop for Keys {
    fn drop(&mut self) {
        unsafe {
            SetKeyboardState(&self.0).unwrap();
        }
    }
}
#[test]
fn live_add_art_exact_shortcut_repeat_menu_and_lifecycle_single_slot() {
    unsafe {
        let environment = Environment::new();
        let mut app = environment.app();
        let (hwnd, _) = live(&mut app, true);
        let keys = Keys::new();
        keys.set(&[VK_SHIFT]);
        SendMessageW(hwnd, WM_KEYDOWN, Some(WPARAM(0x31)), Some(LPARAM(1 << 30)));
        assert!(!app.live_sessions[0].art_pending());
        SendMessageW(hwnd, WM_KEYDOWN, Some(WPARAM(0x31)), None);
        assert!(app.live_sessions[0].art_pending());
        assert!(!app.live_sessions[0].persistence_idle() && !app.live_sessions[0].exit_idle());
        assert!(!crate::desktop::application::restore_batch::guard::is_idle(
            &app
        ));
        assert!(app.live_sessions[0].take_art_request());
        assert!(!app.live_sessions[0].take_art_request());
        for held in [
            &[VK_SHIFT, VK_CONTROL][..],
            &[VK_SHIFT, VK_MENU],
            &[VK_SHIFT, VK_LWIN],
            &[],
        ] {
            keys.set(held);
            SendMessageW(hwnd, WM_KEYDOWN, Some(WPARAM(0x31)), None);
            assert!(!app.live_sessions[0].art_pending());
        }
        SendMessageW(hwnd, WM_SYSCOMMAND, Some(WPARAM(0x1f80)), None);
        assert!(app.live_sessions[0].take_art_request());
        app.shutdown();
    }
}
#[test]
fn live_add_art_no_frame_reports_real_input_error_and_capacity_never_leaves_half_child() {
    unsafe {
        let environment = Environment::new();
        for full in [false, true] {
            let mut app = environment.app();
            configured(&app);
            let (_, unit) = live(&mut app, false);
            if full {
                app.workspace
                    .add_units(
                        (app.workspace.units().len()..crate::workspace::MAX_UNITS).map(|index| {
                            crate::workspace_id::UnitId::parse(&format!(
                                "live-art-capacity-{index}"
                            ))
                            .unwrap()
                        }),
                    )
                    .unwrap();
                let topology = app.workspace.topology_stamp();
                assert!(add(&mut app, &unit).is_err());
                assert!(topology.matches(&app.workspace));
                assert_eq!(app.documents.len(), 1);
                assert!(app.workspace.links().is_empty());
            } else {
                assert!(add(&mut app, &unit).unwrap());
                app.start_requested_preparations(false);
                // 原自动链在首帧前静候，不制造错误或输入；显式手动运行才报告缺帧。
                assert!(app.documents[1].state.preparation_error.borrow().is_none());
                assert!(app.documents[1]
                    .state
                    .pending_preparation
                    .borrow()
                    .is_none());
                assert!(app.documents[1].state.pending_execution.borrow().is_none());
                assert!(app.input_jobs.is_empty() && app.execution_jobs.is_empty());
                let child = &app.documents[1];
                crate::desktop::execution_start::request(child.window.0, &child.state).unwrap();
                app.start_requested_preparations(false);
                assert_eq!(
                    app.documents[1].state.preparation_error.borrow().as_deref(),
                    Some("Live 尚无可用画面")
                );
                assert!(app.input_jobs.is_empty() && app.execution_jobs.is_empty());
            }
            assert!(app.live_sessions[0].is_alive());
            app.shutdown();
        }
    }
}
#[test]
fn live_add_art_automatic_child_input_reaches_real_tls_execute_with_original_pixels() {
    unsafe {
        let environment = Environment::new();
        let _input = crate::desktop::loom_input_job::TEST_SERIAL.lock().unwrap();
        let mut app = environment.app();
        let (listener, config) = super::execution::select(&app);
        app.catalog.borrow_mut().loaded = Some(
            app.documents[0]
                .state
                .selected_art
                .borrow_mut()
                .take()
                .unwrap()
                .catalog,
        );
        let (_, unit) = live(&mut app, true);
        assert!(add(&mut app, &unit).unwrap());
        app.start_requested_preparations(false);
        let deadline = Instant::now() + Duration::from_secs(5);
        while app.documents[1]
            .state
            .pending_preparation
            .borrow()
            .is_some()
        {
            app.input_jobs.poll();
            app.retire_preparations();
            app.collect_prepared_inputs();
            assert!(Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(1));
        }
        assert!(app.documents[1].state.pending_execution.borrow().is_some());
        let server = crate::desktop::application::execution_test_peer::serve(listener, config, 3);
        app.start_pending_executions();
        while app.documents[1].state.execution_results.borrow().status()
            != crate::desktop::loom_result_state::Status::Completed
        {
            app.execution_jobs.poll();
            app.collect_execution_updates();
            app.collect_execution_results();
            assert!(Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(1));
        }
        server.join().unwrap();
        assert_eq!(
            app.documents[1]
                .state
                .execution_results
                .borrow()
                .save_value_outputs()
                .unwrap()["output"],
            serde_json::json!(44)
        );
        assert!(app.live_sessions[0].is_alive());
        app.shutdown();
    }
}

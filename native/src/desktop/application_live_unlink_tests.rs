//! 隐藏 Live 的系统命令和注入菜单选择贯通断线、显示退休、保存及过期保护。
use super::*;
use crate::desktop::link_menu::FIRST;

#[test]
fn live_unlink_retires_prepared_execution_before_any_remote_request() {
    unsafe {
        let environment = Environment::new();
        let _input = crate::desktop::loom_input_job::TEST_SERIAL.lock().unwrap();
        let mut app = environment.app();
        let (listener, _) = execution::select(&app);
        let (_, unit) = live(&mut app, true);
        execution::connect(&mut app, &unit);
        let doc = &app.documents[0];
        crate::desktop::execution_start::request(doc.window.0, &doc.state).unwrap();
        app.start_requested_preparations(false);
        let deadline = Instant::now() + Duration::from_secs(5);
        while app.documents[0]
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
        let request = app.documents[0]
            .state
            .pending_execution
            .borrow()
            .as_ref()
            .unwrap()
            .request
            .clone();
        assert!(app
            .unlink_live_with(&unit, |_, _, snapshot| Ok(snapshot.request(FIRST)))
            .unwrap());
        app.cancel_requested_executions();
        assert!(app.documents[0].state.pending_execution.borrow().is_none());
        assert!(!app
            .executions
            .is_latest(&app.documents[0].state.unit_id, &request));
        app.start_pending_executions();
        assert!(app.execution_jobs.is_empty());
        assert_eq!(
            listener.accept().unwrap_err().kind(),
            std::io::ErrorKind::WouldBlock
        );
        app.shutdown_live().unwrap();
    }
}

#[test]
fn live_unlink_command_removes_only_selected_edge_retires_display_and_saves() {
    unsafe {
        let environment = Environment::new();
        let mut app = environment.app();
        let (hwnd, unit) = live(&mut app, true);
        execution::connect(&mut app, &unit);
        app.add_window(false).unwrap();
        assert!(app
            .create_live_link_with(&unit, |_, _, _, _| Ok(Some((1, 0, 0))))
            .unwrap());
        let edges = app.workspace.links().to_vec();
        let deadline = Instant::now() + Duration::from_secs(5);
        while app
            .documents
            .iter()
            .any(|doc| doc.state.resolved_display.borrow().is_none())
        {
            app.sync_display_frames();
            assert!(Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(1));
        }
        SendMessageW(hwnd, WM_SYSCOMMAND, Some(WPARAM(0x1f90)), Some(LPARAM(0)));
        assert!(app.live_sessions[0].take_unlink_request());
        assert!(!app.live_sessions[0].take_unlink_request());
        assert!(app
            .unlink_live_with(&unit, |app, owner, snapshot| {
                assert!(app.modals.blocked());
                assert_eq!(owner, hwnd);
                assert_eq!(app.workspace.links(), edges);
                Ok(snapshot.request(FIRST))
            })
            .unwrap());
        assert!(!app.modals.blocked());
        assert_eq!(app.workspace.links(), &edges[1..]);
        app.sync_display_frames();
        assert!(app.documents[0].state.resolved_display.borrow().is_none());
        assert!(app.documents[1].state.resolved_display.borrow().is_some());
        assert!(app.live_sessions[0].is_alive());
        app.request_save_candidate().unwrap();
        let saved = save::prepared(&mut app).unwrap();
        let decoded = crate::session_document::decode(&saved.candidate.bytes).unwrap();
        assert_eq!(decoded.workspace.links(), &edges[1..]);
        app.shutdown_live().unwrap();
    }
}

#[test]
fn live_unlink_cancel_error_empty_and_foreign_selection_do_not_mutate() {
    unsafe {
        let environment = Environment::new();
        let mut app = environment.app();
        let (hwnd, unit) = live(&mut app, true);
        // 无连线的系统命令经过完整应用分派后直接返回，不打开菜单。
        SendMessageW(hwnd, WM_SYSCOMMAND, Some(WPARAM(0x1f90)), Some(LPARAM(0)));
        app.pending_link_changes();
        assert!(!app.live_sessions[0].take_unlink_request());
        assert!(!app
            .unlink_live_with(&unit, |_, owner, snapshot| snapshot.select(owner))
            .unwrap());
        execution::connect(&mut app, &unit);
        let original = app.workspace.links().to_vec();
        for mode in 0..4 {
            let result = app.unlink_live_with(&unit, |_, _, snapshot| match mode {
                0 => Ok(None),
                1 => Ok(snapshot.request(usize::MAX)),
                2 => Err(Error::new(E_FAIL, "injected unlink menu error")),
                _ => {
                    let mut request = snapshot.request(FIRST).unwrap();
                    request.link.from_port = "foreign".into();
                    Ok(Some(request))
                }
            });
            if mode < 2 {
                assert!(!result.unwrap());
            } else {
                assert!(result.is_err());
            }
            assert_eq!(app.workspace.links(), original);
            assert!(!app.modals.blocked());
        }
        app.shutdown_live().unwrap();
    }
}

#[test]
fn live_unlink_stale_closed_locked_hidden_busy_and_replaced_requests_are_rejected() {
    unsafe {
        let environment = Environment::new();
        for mode in 0..10 {
            let mut app = environment.app();
            if mode == 9 {
                app.add_window(false).unwrap();
            }
            let (hwnd, unit) = live(&mut app, true);
            execution::connect(&mut app, &unit);
            let edge = app.workspace.links()[0].clone();
            let result = app.unlink_live_with(&unit, |app, _, snapshot| {
                let request = snapshot.request(FIRST);
                match mode {
                    0 => {
                        DestroyWindow(hwnd).unwrap();
                    }
                    1 => {
                        DestroyWindow(app.documents[0].window.0).unwrap();
                    }
                    2 => {
                        app.workspace = crate::workspace::Workspace::with_initial(unit.clone());
                        app.workspace.add_units([edge.to_unit.clone()]).unwrap();
                        app.workspace.connect(edge.clone()).unwrap();
                    }
                    3 => {
                        app.workspace.remove_link(&edge.id);
                        app.workspace.connect(edge.clone()).unwrap();
                    }
                    4..=6 => {
                        let id = crate::workspace_id::UnitId::parse("unlink-group").unwrap();
                        app.workspace
                            .upsert_group(crate::workspace::Group {
                                id: id.clone(),
                                name: "断线保护".into(),
                                hidden: mode == 6,
                                locked: mode != 6,
                            })
                            .unwrap();
                        let member = if mode == 4 { &edge.to_unit } else { &unit };
                        app.workspace
                            .assign_group(std::slice::from_ref(member), Some(&id))
                            .unwrap();
                    }
                    7 => app.exit_requested.set(true),
                    8 => app.documents[0].state.file_dialog.set(true),
                    _ => app.documents[1].state.file_dialog.set(true),
                }
                Ok(request)
            });
            assert!(result.is_err(), "mode {mode}");
            assert_eq!(app.workspace.links(), &[edge]);
            if mode >= 8 {
                // 被注入的另一窗口模态仍归原 owner，不应被本次菜单 guard 清除。
                assert!(app.modals.blocked());
                app.documents[mode - 8].state.file_dialog.set(false);
            }
            assert!(!app.modals.blocked());
            if matches!(mode, 5 | 6) {
                app.sync_groups();
                SendMessageW(hwnd, WM_SYSCOMMAND, Some(WPARAM(0x1f90)), Some(LPARAM(0)));
                assert!(!app.live_sessions[0].take_unlink_request());
                assert!(app
                    .unlink_live_with(&unit, |_, _, _| panic!(
                        "locked or hidden owner opened menu"
                    ))
                    .is_err());
                assert!(app
                    .create_live_link_with(&unit, |_, _, _, _| panic!(
                        "locked or hidden owner opened dialog"
                    ))
                    .is_err());
            }
            app.shutdown_live().unwrap();
        }
        let mut app = environment.app();
        let (_, unit) = live(&mut app, true);
        let blocked = std::rc::Rc::new(std::cell::Cell::new(true));
        app.modals.register(&blocked);
        assert!(app
            .unlink_live_with(&unit, |_, _, _| panic!("busy menu opened"))
            .is_err());
        drop(blocked);
        app.shutdown_live().unwrap();
    }
}

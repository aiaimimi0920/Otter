//! 真实 Live 系统命令/隐藏选择控件加注入选择，验证连线事务；不冒充可见模态人工验收。
use super::*;
unsafe fn request(app: &Application, hwnd: HWND) {
    SendMessageW(hwnd, WM_SYSCOMMAND, Some(WPARAM(0x1fa0)), Some(LPARAM(0)));
    assert!(app.live_sessions[0].take_link_request());
    assert!(!app.live_sessions[0].take_link_request());
}
#[test]
fn live_link_outgoing_menu_dialog_commit_displays_and_saves_real_edge() {
    unsafe {
        let environment = Environment::new();
        let mut app = environment.app();
        app.add_window(false).unwrap();
        let (hwnd, unit) = live(&mut app, true);
        request(&app, hwnd);
        assert!(app
            .create_live_link_with(&unit, |app, owner, catalog, source| {
                assert!(app.modals.blocked());
                assert_eq!(owner, hwnd);
                assert!(app.workspace.links().is_empty());
                crate::desktop::link_create::tests::verify_outgoing(catalog.clone(), source);
                Ok(Some((1, 0, 0)))
            })
            .unwrap());
        assert!(!app.modals.blocked());
        let edge = app.workspace.links()[0].clone();
        assert_eq!(edge.from_unit, unit);
        assert_eq!(edge.to_unit, app.documents[1].state.unit_id);
        assert_eq!((&*edge.from_port, &*edge.to_port), ("output", "image"));
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            app.sync_display_frames();
            if app.documents[1].state.resolved_display.borrow().is_some() {
                break;
            }
            assert!(Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(1));
        }
        assert_eq!(
            &*app.documents[1]
                .state
                .resolved_display
                .borrow()
                .as_ref()
                .unwrap()
                .image
                .pixels,
            &[0, 0, 255, 255].repeat(4)
        );
        app.request_save_candidate().unwrap();
        let saved = super::save::prepared(&mut app).unwrap();
        let decoded = crate::session_document::decode(&saved.candidate.bytes).unwrap();
        assert_eq!(decoded.workspace.links(), &[edge]);
        app.shutdown_live().unwrap();
    }
}
#[test]
fn live_link_cancel_invalid_selection_and_dialog_error_leave_no_partial_edge() {
    unsafe {
        let environment = Environment::new();
        let mut app = environment.app();
        let (_, unit) = live(&mut app, true);
        for mode in 0..5 {
            let result = app.create_live_link_with(&unit, |_, _, _, source| match mode {
                0 => Ok(None),
                1 => Ok(Some((usize::MAX, 0, 0))),
                2 => Ok(Some((source, 0, 0))),
                3 => Ok(Some((0, usize::MAX, 0))),
                _ => Err(Error::new(E_FAIL, "injected dialog failure")),
            });
            if mode == 0 {
                assert!(!result.unwrap());
            } else {
                assert!(result.is_err());
            }
            assert!(app.workspace.links().is_empty());
            assert!(!app.modals.blocked());
        }
        app.shutdown_live().unwrap();
    }
}
#[test]
fn live_link_confirmation_rejects_stale_or_locked_owners_and_busy_scope() {
    unsafe {
        let environment = Environment::new();
        for mode in 0..7 {
            let mut app = environment.app();
            let (hwnd, unit) = live(&mut app, true);
            assert!(app
                .create_live_link_with(&unit, |app, _, _, _| {
                    match mode {
                        0 => {
                            DestroyWindow(hwnd).unwrap();
                        }
                        1 => {
                            DestroyWindow(app.documents[0].window.0).unwrap();
                        }
                        2 => {
                            app.workspace = crate::workspace::Workspace::with_initial(
                                app.documents[0].state.unit_id.clone(),
                            );
                        }
                        3 => {
                            let flag = &app.documents[0].state.accept_upstream;
                            flag.set(!flag.get());
                        }
                        4 | 5 => {
                            let group = crate::workspace_id::UnitId::parse("locked-link").unwrap();
                            app.workspace
                                .upsert_group(crate::workspace::Group {
                                    id: group.clone(),
                                    name: "锁定".into(),
                                    hidden: false,
                                    locked: true,
                                })
                                .unwrap();
                            let member = if mode == 4 {
                                &app.documents[0].state.unit_id
                            } else {
                                &unit
                            };
                            app.workspace
                                .assign_group(std::slice::from_ref(member), Some(&group))
                                .unwrap();
                        }
                        _ => app.exit_requested.set(true),
                    }
                    Ok(Some((0, 0, 0)))
                })
                .is_err());
            assert!(app.workspace.links().is_empty());
            assert!(!app.modals.blocked());
            app.shutdown_live().unwrap();
        }
        let mut app = environment.app();
        let (_, unit) = live(&mut app, true);
        let blocked = std::rc::Rc::new(std::cell::Cell::new(true));
        app.modals.register(&blocked);
        assert!(app
            .create_live_link_with(&unit, |_, _, _, _| panic!(
                "busy scope must not open dialog"
            ))
            .is_err());
        drop(blocked);
        app.shutdown_live().unwrap();
    }
}

//! 隐藏 Application 的独立子 Art/自动连源事务；不运行真实设备、用户服务或全局输入。
use super::*;
use desktop::{
    node_menu::art_picker::{model::Snapshot, tests as fixture},
    Apartment,
};
unsafe fn app() -> Application {
    let (instance, doc) = fixture::document();
    let app = Application::new(instance, doc);
    app.catalog.borrow_mut().loaded = Some(fixture::catalog(|_| {}));
    app
}
unsafe fn request(app: &Application) -> Request {
    let doc = &app.documents[0];
    Snapshot::capture(doc.window.0, &doc.state)
        .request(0, &doc.state)
        .unwrap()
}
#[test]
fn new_art_is_independent_connected_and_defaults_upstream_request_do_not_mutate_source() {
    unsafe {
        let _apartment = Apartment::new().unwrap();
        let mut app = app();
        let source = app.documents[0].state.unit_id.clone();
        let pixels = app.documents[0]
            .state
            .canvas
            .borrow()
            .image
            .as_ref()
            .unwrap()
            .pixels
            .clone();
        let pending = request(&app);
        app.add_connected_art(&source, pending).unwrap();
        assert_eq!(app.documents.len(), 2);
        assert_eq!(app.workspace.links().len(), 1);
        assert!(!app.documents[0].state.is_art());
        assert!(pixels.same_storage(
            &app.documents[0]
                .state
                .canvas
                .borrow()
                .image
                .as_ref()
                .unwrap()
                .pixels
        ));
        let child = &app.documents[1];
        assert_ne!(child.state.unit_id, source);
        assert!(child.state.is_art());
        let selection = child.state.selected_art.borrow();
        let selected = selection.as_ref().unwrap();
        assert_eq!(
            selected.parameters.get("amount"),
            Some(Some(&serde_json::json!(42)))
        );
        assert_eq!(app.workspace.links()[0].from_unit, source);
        assert_eq!(app.workspace.links()[0].to_unit, child.state.unit_id);
        assert_eq!(app.workspace.links()[0].to_port, "pixels");
        assert_eq!(
            child.state.pending_start.borrow().as_ref().unwrap().trigger,
            crate::graph_execution_config::Trigger::Upstream
        );
        assert!(app.input_jobs.is_empty() && app.execution_jobs.is_empty());
        drop(selection);
        app.shutdown();
    }
}
#[test]
fn source_notify_and_child_upstream_settings_are_not_bypassed_by_manual_creation() {
    unsafe {
        let _apartment = Apartment::new().unwrap();
        for setting in 0..3 {
            let mut app = app();
            if setting == 0 {
                let state = &app.documents[0].state;
                let catalog = app.catalog.borrow().loaded.clone().unwrap();
                let mut execution = catalog
                    .handshake
                    .capability(0)
                    .unwrap()
                    .execution_config(None);
                execution.notify_downstream = false;
                *state.selected_art.borrow_mut() = Some(desktop::capability_selection::Selection {
                    execution,
                    identity: desktop::document_state::new_unit_id().unwrap(),
                    parameters: crate::loom_parameter_state::Parameters::new(
                        &catalog.handshake.capability(0).unwrap(),
                    )
                    .unwrap(),
                    catalog,
                });
            }
            let source = app.documents[0].state.unit_id.clone();
            let pending = request(&app);
            app.add_connected_art(&source, pending).unwrap();
            let child = &app.documents[1].state;
            if setting == 0 {
                assert!(child.pending_start.borrow().is_none());
                app.documents[0]
                    .state
                    .selected_art
                    .borrow_mut()
                    .as_mut()
                    .unwrap()
                    .execution
                    .notify_downstream = true;
            } else {
                assert!(child.pending_start.borrow_mut().take().is_some());
                let mut selected = child.selected_art.borrow_mut();
                let execution = &mut selected.as_mut().unwrap().execution;
                if setting == 1 {
                    execution.upstream_driven = false;
                } else {
                    execution.listen_upstream = false;
                }
            }
            app.propagate_created_child(&source);
            assert_eq!(child.pending_start.borrow().is_some(), setting == 0);
            app.shutdown();
        }
    }
}
#[test]
fn stale_invalid_parameters_and_capacity_failure_leave_no_child_or_half_link() {
    unsafe {
        let _apartment = Apartment::new().unwrap();
        for mode in 0..3 {
            let mut app = app();
            if mode == 1 {
                app.catalog.borrow_mut().loaded = Some(fixture::catalog(|raw| {
                    raw["capabilities"]["artDefinitions"][0]["parameters"] =
                        serde_json::json!([{"id":"bad\nname","default":0}])
                }));
            }
            let source = app.documents[0].state.unit_id.clone();
            let pending = request(&app);
            if mode == 0 {
                app.catalog.borrow_mut().loaded = Some(fixture::catalog(|_| {}));
            }
            if mode == 2 {
                app.workspace
                    .add_units((1..crate::workspace::MAX_UNITS).map(|index| {
                        crate::workspace_id::UnitId::parse(&format!("capacity-{index}")).unwrap()
                    }))
                    .unwrap();
            }
            let before = app.workspace.topology_stamp();
            let registered = app.capture_windows.borrow().len();
            assert!(app.add_connected_art(&source, pending).is_err());
            assert!(before.matches(&app.workspace));
            assert_eq!(app.documents.len(), 1);
            assert_eq!(app.capture_windows.borrow().len(), registered);
            assert!(!app.modals.blocked());
            assert!(!app.documents[0].state.is_art());
            app.shutdown();
        }
    }
}
#[test]
fn synchronous_catalog_replacement_reclaims_unpublished_window_and_keeps_source() {
    unsafe {
        let _apartment = Apartment::new().unwrap();
        let mut app = app();
        let source = app.documents[0].state.unit_id.clone();
        let pending = request(&app);
        let before = app.workspace.topology_stamp();
        fixture::replace_during_create(app.catalog.clone());
        assert!(app.add_connected_art(&source, pending).is_err());
        assert!(before.matches(&app.workspace));
        assert_eq!(app.documents.len(), 1);
        assert_eq!(app.capture_windows.borrow().len(), 2);
        assert!(!IsWindow(Some(fixture::candidate())).as_bool());
        assert!(!app.modals.blocked());
        app.shutdown();
    }
}
#[test]
fn queued_art_intent_blocks_restore_and_durable_system_query_until_consumed() {
    unsafe {
        let _apartment = Apartment::new().unwrap();
        let mut app = app();
        let root = std::path::Path::new(
            "C:/Users/Public/nas_home/AI/GameEditor/linshi/otter-native-add-art-tests",
        )
        .join(format!(
            "{:?}",
            windows::Win32::System::Com::CoCreateGuid().unwrap()
        ));
        std::fs::create_dir_all(&root).unwrap();
        app.install_workspace_checkpoint(
            Some(desktop::workspace_profile::Profile::open(&root).unwrap()),
            true,
        )
        .unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        loop {
            app.pending_workspace_save_with(|_, _, message, warning| {
                assert!(!warning, "{message}")
            });
            if app.save_job.is_none() {
                break;
            }
            assert!(std::time::Instant::now() < deadline);
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
        let hwnd = app.documents[0].window.0;
        let query = |app: &Application| {
            desktop::session_end::with_query(
                &app.session_end,
                || app.session_end_ready(),
                || SendMessageW(hwnd, WM_QUERYENDSESSION, None, None).0,
            )
        };
        assert_eq!(query(&app), 1);
        SendMessageW(hwnd, WM_ENDSESSION, Some(WPARAM(0)), None);
        let pending = request(&app);
        let doc = &app.documents[0];
        pending.queue(doc.window.0, &doc.state).unwrap();
        assert!(restore_batch::guard::Guard::capture(&app).is_err());
        assert!(doc.state.art_request.borrow().is_some());
        assert_eq!(query(&app), 0);
        SendMessageW(hwnd, WM_ENDSESSION, Some(WPARAM(0)), None);
        app.process_art_requests(false).unwrap();
        assert_eq!(app.documents.len(), 2);
        app.shutdown();
    }
}

fn local(app: &mut Application) {
    app.catalog.borrow_mut().loaded = Some(fixture::catalog(|raw| {
        raw["capabilities"]["artDefinitions"][0]["metadata"] =
            serde_json::json!({"capabilities":{"surface":{"variants":[{"runtime":"javascript"}]}}})
    }));
    app.authorization.borrow_mut().origin = Some("https://loom.example".into());
    app.art_issuers = crate::art_local_requests::tests::authority();
}
#[test]
fn local_candidate_rechecks_catalog_explicit_origin_and_session_before_starting_worker() {
    unsafe {
        let _apartment = Apartment::new().unwrap();
        for change in 0..3 {
            let mut app = app();
            local(&mut app);
            let source = app.documents[0].state.unit_id.clone();
            let pending = request(&app);
            let before = app.workspace.topology_stamp();
            let catalog = app.catalog.clone();
            let authorization = app.authorization.clone();
            let control = app.session_end.clone();
            let hwnd = app.documents[0].window.0;
            fixture::during_create(move || match change {
                0 => catalog.borrow_mut().loaded = Some(fixture::catalog(|_| {})),
                1 => authorization.borrow_mut().origin = Some("https://changed.invalid".into()),
                _ => {
                    desktop::session_end::with_query(
                        &control,
                        || true,
                        || {
                            assert_eq!(
                                SendMessageW(hwnd, WM_QUERYENDSESSION, None, None),
                                LRESULT(1)
                            );
                        },
                    );
                }
            });
            assert!(app.add_connected_art(&source, pending).is_err());
            assert!(before.matches(&app.workspace));
            assert_eq!(app.documents.len(), 1);
            assert_eq!(app.capture_windows.borrow().len(), 2);
            assert!(!app.local_arts.busy());
            assert!(!IsWindow(Some(fixture::candidate())).as_bool());
            desktop::session_end::dispatch(
                hwnd,
                &app.documents[0].state,
                WM_ENDSESSION,
                WPARAM(0),
                LPARAM(0),
            );
            app.shutdown();
        }
    }
}
#[test]
fn adding_same_local_art_forces_an_independent_child_instead_of_restarting_source() {
    unsafe {
        let _apartment = Apartment::new().unwrap();
        let mut app = app();
        local(&mut app);
        let art = app
            .catalog
            .borrow()
            .loaded
            .as_ref()
            .unwrap()
            .handshake
            .capability(0)
            .unwrap()
            .id
            .to_owned();
        app.documents[0].state.local_art_id = Some(art.clone().into());
        let source = app.documents[0].state.unit_id.clone();
        let pending = request(&app);
        app.add_connected_art(&source, pending).unwrap();
        assert_eq!(app.documents.len(), 2);
        assert_eq!(app.documents[0].state.unit_id, source);
        assert_eq!(
            app.documents[0].state.local_art_id.as_deref(),
            Some(art.as_str())
        );
        assert_eq!(
            app.documents[1].state.local_art_id.as_deref(),
            Some(art.as_str())
        );
        let link = &app.workspace.links()[0];
        assert_eq!(link.from_unit, source);
        assert_eq!(link.to_unit, app.documents[1].state.unit_id);
        assert_eq!(link.from_port, "output_image");
        app.shutdown();
    }
}

//! 隐藏 Live/WARP 经生产连线、输入 worker 和回环 TLS 对端执行；不访问用户服务。
use super::*;
use crate::desktop::application::execution_test_peer as peer;
pub(super) fn select(app: &Application) -> (std::net::TcpListener, Arc<rustls::ServerConfig>) {
    let (listener, identity, config) = crate::loom_bridge_client::tests::fixture();
    listener.set_nonblocking(true).unwrap();
    let mut value = crate::loom_handshake::tests::fixture();
    value["capabilities"]["artDefinitions"][0]["inputs"] =
        serde_json::json!([{"name":"input_image","type":"image","required":true}]);
    let handshake = crate::loom_handshake::response(&serde_json::to_vec(&value).unwrap())
        .unwrap()
        .unwrap();
    let parameters =
        crate::loom_parameter_state::Parameters::new(&handshake.capability(0).unwrap()).unwrap();
    *app.documents[0].state.selected_art.borrow_mut() =
        Some(crate::desktop::capability_selection::Selection {
            execution: Default::default(),
            identity: crate::desktop::document_state::new_unit_id().unwrap(),
            parameters,
            catalog: std::rc::Rc::new(crate::desktop::capability_job::Loaded {
                identity,
                handshake,
            }),
        });
    (listener, config)
}
pub(super) unsafe fn connect(app: &mut Application, unit: &crate::workspace_id::UnitId) {
    app.sync_link_menus();
    let doc = &app.documents[0];
    let catalog = doc.state.link_catalog.borrow().clone().unwrap();
    let index = catalog
        .endpoints
        .iter()
        .position(|endpoint| &endpoint.unit == unit)
        .unwrap();
    assert!(catalog.endpoints[index].inputs.is_empty());
    assert!(catalog
        .queue(doc.window.0, &doc.state, index, 0, 0)
        .unwrap());
    app.pending_link_changes();
    assert_eq!(app.workspace.links().len(), 1);
    assert_eq!(&app.workspace.links()[0].from_unit, unit);
}
unsafe fn start(app: &mut Application) {
    let doc = &app.documents[0];
    crate::desktop::execution_start::request(doc.window.0, &doc.state).unwrap();
    app.start_requested_preparations(false);
}
fn prepare(app: &mut Application) {
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
        assert!(Instant::now() < deadline, "Live prepare timeout");
        std::thread::sleep(Duration::from_millis(1));
    }
}
#[test]
fn live_execution_menu_frame_reaches_tls_peer_and_result_returns() {
    unsafe {
        let environment = Environment::new();
        let _input = crate::desktop::loom_input_job::TEST_SERIAL.lock().unwrap();
        let mut app = environment.app();
        let (listener, config) = select(&app);
        let (_, unit) = live(&mut app, true);
        connect(&mut app, &unit);
        start(&mut app);
        assert_eq!(app.input_jobs.len(), 1);
        prepare(&mut app);
        assert!(app.documents[0].state.pending_execution.borrow().is_some());
        assert_eq!(
            listener.accept().unwrap_err().kind(),
            std::io::ErrorKind::WouldBlock
        );
        let server = peer::serve(listener, config, 3);
        app.start_pending_executions();
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            app.execution_jobs.poll();
            app.collect_execution_updates();
            app.collect_execution_results();
            if app.documents[0].state.execution_results.borrow().status()
                == crate::desktop::loom_result_state::Status::Completed
            {
                break;
            }
            assert!(Instant::now() < deadline, "Live execution result timeout");
            std::thread::sleep(Duration::from_millis(1));
        }
        server.join().unwrap();
        let results = app.documents[0].state.execution_results.borrow();
        let crate::desktop::loom_output_batch::Output::Value(value) = &results.ports()[0].value
        else {
            panic!("expected scalar")
        };
        assert_eq!(*value, serde_json::json!(44));
        drop(results);
        // 同一生产断线入口也能移除 Live 边，不因缺少 DocumentWindow 拒绝。
        app.sync_link_menus();
        let doc = &app.documents[0];
        let menu = doc.state.link_menu.borrow().clone().unwrap();
        assert!(menu
            .queue(doc.window.0, &doc.state, crate::desktop::link_menu::FIRST)
            .unwrap());
        app.pending_link_changes();
        assert!(app.workspace.links().is_empty());
        app.shutdown_live().unwrap();
    }
}
#[test]
fn live_execution_closed_owner_rejects_prepared_input_before_network() {
    unsafe {
        let environment = Environment::new();
        let _input = crate::desktop::loom_input_job::TEST_SERIAL.lock().unwrap();
        for after_prepare in [false, true] {
            let mut app = environment.app();
            let (listener, _) = select(&app);
            let (hwnd, unit) = live(&mut app, true);
            connect(&mut app, &unit);
            start(&mut app);
            if after_prepare {
                prepare(&mut app);
            }
            DestroyWindow(hwnd).unwrap();
            if !after_prepare {
                prepare(&mut app);
            }
            app.start_pending_executions();
            assert!(app.execution_jobs.is_empty());
            assert!(app.documents[0].state.pending_execution.borrow().is_none());
            assert_eq!(
                listener.accept().unwrap_err().kind(),
                std::io::ErrorKind::WouldBlock
            );
            app.shutdown_live().unwrap();
        }
    }
}
#[test]
fn live_execution_no_frame_and_old_catalog_never_fake_input() {
    unsafe {
        let environment = Environment::new();
        let mut app = environment.app();
        let (listener, _) = select(&app);
        let (hwnd, unit) = live(&mut app, false);
        connect(&mut app, &unit);
        start(&mut app);
        assert!(app.input_jobs.is_empty());
        assert_eq!(
            app.documents[0].state.preparation_error.borrow().as_deref(),
            Some("Live 尚无可用画面")
        );
        let target = app.documents[0].state.unit_id.clone();
        assert!(Application::image_inputs(
            &app.documents,
            &app.live_sessions,
            &app.workspace,
            &target,
            false,
            None,
            None
        )
        .unwrap()
        .is_none());
        let catalog = app.documents[0]
            .state
            .link_catalog
            .borrow()
            .clone()
            .unwrap();
        DestroyWindow(hwnd).unwrap();
        assert!(!catalog.matches(&app.documents, &app.live_sessions, &app.workspace));
        assert_eq!(
            listener.accept().unwrap_err().kind(),
            std::io::ErrorKind::WouldBlock
        );
        app.shutdown_live().unwrap();
    }
}

#[test]
fn live_execution_link_request_rechecks_source_lock_and_owner() {
    unsafe {
        let environment = Environment::new();
        for mode in 0..3 {
            let mut app = environment.app();
            let (_listener, _) = select(&app);
            let (hwnd, unit) = live(&mut app, false);
            app.sync_link_menus();
            let doc = &app.documents[0];
            let catalog = doc.state.link_catalog.borrow().clone().unwrap();
            let index = catalog
                .endpoints
                .iter()
                .position(|endpoint| endpoint.unit == unit)
                .unwrap();
            assert!(catalog
                .queue(doc.window.0, &doc.state, index, 0, 0)
                .unwrap());
            match mode {
                0 => {
                    DestroyWindow(hwnd).unwrap();
                }
                1 => {
                    app.workspace =
                        crate::workspace::Workspace::with_initial(doc.state.unit_id.clone());
                }
                _ => {
                    let group = crate::workspace_id::UnitId::parse("locked-live").unwrap();
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
            }
            app.pending_link_changes();
            assert!(app.workspace.links().is_empty());
            app.shutdown_live().unwrap();
        }
    }
}

//! 原生模式菜单/原Job/TLS/Checkpoint与原读调度自动提交；不投送真实设备或占前台。
use super::*;
use std::sync::atomic::Ordering;
unsafe fn command(app: &mut Application, files: &Files, command: usize) {
    ui::execute(app.documents[0].window.0, &app.documents[0].state, command).unwrap();
    app.projection_edit_requests(&mut |o| config(o, files.root()));
}
unsafe fn shift(app: &Application, dx: f32) {
    assert!(app.documents[0]
        .state
        .canvas
        .borrow_mut()
        .editor
        .translate(0, Point { x: dx, y: 0.0 })
        .unwrap());
}
unsafe fn sync(app: &mut Application, files: &Files) {
    let state = snapshot(app);
    assert!(app.start_projection_source(
        app.documents[0].state.unit_id.clone(),
        Some(vec![state.projection_id().into()]),
        false,
        &mut |o| config(o, files.root())
    ));
    finish(app, files);
}
#[test]
fn source_mode_menu_waits_real_checkpoint_and_keeps_network_period_edit_for_automatic_submit() {
    let _serial = serial();
    let _com = Apartment::new().unwrap();
    let (files, identity, _) = Files::new();
    let (listener, client, tls, pem) =
        crate::device_session_client::tests::fixture_with_host("127.0.0.1");
    std::fs::write(files.root().join("ca.pem"), pem).unwrap();
    let peer = peer::Peer::start(listener, tls, files.root().to_path_buf());
    unsafe {
        let (_class, mut app) = application(files.root());
        initialized(&mut app, &files, identity, client, 1);
        command(&mut app, &files, ui::editing::TWO_WAY);
        completed(&app);
        shift(&app, 3.0);
        poll(&mut app, &files);
        assert_eq!(snapshot(&app).document().mode(), edit::Mode::TwoWay);
        let id = snapshot(&app)
            .document()
            .objects()
            .keys()
            .next()
            .unwrap()
            .clone();
        assert_eq!(snapshot(&app).document().objects()[&id].value()["x"], 4.0);
        assert_eq!(
            snapshot(&app).editor_view().unwrap().basis_objects()[&id]["x"],
            7.0
        );
        let original = journal(&files)["entries"][0]["pending"]["request"].clone();
        assert_eq!(original["operation"], "mode");
        poll(&mut app, &files);
        assert!(app.projection_edit.busy());
        assert_eq!(
            journal(&files)["entries"][0]["pending"]["request"],
            original
        );
        finish(&mut app, &files);
        assert!(journal(&files)["entries"][0]["pending"].is_null());
        assert_eq!(snapshot(&app).document().objects()[&id].value()["x"], 7.0);
        command(&mut app, &files, ui::editing::ONE_WAY);
        finish(&mut app, &files);
        assert_eq!(snapshot(&app).document().mode(), edit::Mode::OneWay);
        let trace = peer.trace.lock().unwrap();
        assert_eq!(
            trace
                .iter()
                .filter(|r| r["body"]["operation"] == "mode")
                .count(),
            2
        );
        assert_eq!(
            trace
                .iter()
                .filter(|r| r["body"]["operation"] == "apply")
                .count(),
            1
        );
        drop(trace);
        app.shutdown();
    }
}
#[test]
fn source_automatic_failure_and_cancel_never_replay_unknown_request_without_explicit_retry() {
    let _serial = serial();
    let _com = Apartment::new().unwrap();
    for cancel in [false, true] {
        let (files, identity, _) = Files::new();
        let (listener, client, tls, pem) =
            crate::device_session_client::tests::fixture_with_host("127.0.0.1");
        std::fs::write(files.root().join("ca.pem"), pem).unwrap();
        let peer = peer::Peer::start(listener, tls, files.root().to_path_buf());
        unsafe {
            let (_class, mut app) = application(files.root());
            initialized(&mut app, &files, identity, client, 1);
            shift(&app, 2.0);
            peer.fail_apply.store(!cancel, Ordering::Release);
            // 成功的原读链触发Automatic，而不是直接调用新网络入口。
            let state = snapshot(&app);
            assert!(app.start_projection_source(
                app.documents[0].state.unit_id.clone(),
                Some(vec![state.projection_id().into()]),
                false,
                &mut |o| config(o, files.root())
            ));
            let end = Instant::now() + Duration::from_secs(10);
            loop {
                poll(&mut app, &files);
                settle(&mut app);
                // 原日志两段rename间存在可见间隔；不能在worker写入时盲读磁盘作为同步器。
                // 对端已收到请求证明写前持久化完成；下一poll之前取消，仍保原pending。
                if peer
                    .trace
                    .lock()
                    .unwrap()
                    .iter()
                    .any(|row| row["body"]["operation"] == "apply")
                {
                    break;
                }
                assert!(Instant::now() < end);
            }
            if cancel {
                command(&mut app, &files, ui::editing::CANCEL);
            }
            finish(&mut app, &files);
            let request = journal(&files)["entries"][0]["pending"]["request"].clone();
            assert_eq!(request["operation"], "apply");
            let calls = peer.trace.lock().unwrap().len();
            for _ in 0..2 {
                app.automatic_projection_edit(app.documents[0].state.unit_id.clone(), &mut |o| {
                    config(o, files.root())
                });
                assert!(!app.projection_edit.busy());
            }
            assert_eq!(peer.trace.lock().unwrap().len(), calls);
            command(&mut app, &files, ui::editing::SUBMIT);
            finish(&mut app, &files);
            assert!(journal(&files)["entries"][0]["pending"].is_null());
            let trace = peer.trace.lock().unwrap();
            assert!(trace
                .iter()
                .filter(|r| r["body"]["operation"] == "apply")
                .all(|r| r["body"] == request));
            drop(trace);
            app.shutdown();
        }
    }
}
#[test]
fn source_successful_read_reclaims_receipt_budget_through_original_checkpoint_operation() {
    let _serial = serial();
    let _com = Apartment::new().unwrap();
    let (files, identity, _) = Files::new();
    let (listener, client, tls, pem) =
        crate::device_session_client::tests::fixture_with_host("127.0.0.1");
    std::fs::write(files.root().join("ca.pem"), pem).unwrap();
    let peer = peer::Peer::start(listener, tls, files.root().to_path_buf());
    unsafe {
        let (_class, mut app) = application(files.root());
        initialized(&mut app, &files, identity, client, 1);
        peer.receipts.store(224, Ordering::Release);
        sync(&mut app, &files);
        assert_eq!(snapshot(&app).document().revision(), 226);
        assert_eq!(snapshot(&app).document().checkpoint_revision(), 226);
        assert_eq!(snapshot(&app).document().receipt_count(), 0);
        assert!(journal(&files)["entries"][0]["pending"].is_null());
        let trace = peer.trace.lock().unwrap();
        assert_eq!(
            trace
                .iter()
                .filter(|r| r["body"]["operation"] == "checkpoint")
                .count(),
            1
        );
        assert!(!trace.iter().any(|r| r["body"]["operation"] == "apply"));
        drop(trace);
        app.shutdown();
    }
}
#[test]
fn source_lost_mode_reply_survives_real_reopen_and_only_explicit_submit_retries_original_bytes() {
    let _serial = serial();
    let _com = Apartment::new().unwrap();
    let (files, identity, _) = Files::new();
    let (listener, client, tls, pem) =
        crate::device_session_client::tests::fixture_with_host("127.0.0.1");
    let origin = client.origin().to_owned();
    std::fs::write(files.root().join("ca.pem"), pem).unwrap();
    let peer = peer::Peer::start(listener, tls, files.root().to_path_buf());
    unsafe {
        let (_class, mut app) = application(files.root());
        initialized(&mut app, &files, identity, client, 1);
        peer.lose_mode.store(true, Ordering::Release);
        command(&mut app, &files, ui::editing::TWO_WAY);
        completed(&app);
        poll(&mut app, &files);
        assert!(!app.projection_edit.busy());
        assert_eq!(snapshot(&app).document().mode(), edit::Mode::OneWay);
        let original = journal(&files)["entries"][0]["pending"]["request"].clone();
        assert_eq!(original["operation"], "mode");
        app.shutdown();
        drop(app);
        drain_messages();
        let instance = GetModuleHandleW(None).unwrap().into();
        let doc =
            DocumentWindow::create(instance, State::new(None, false).unwrap(), false).unwrap();
        let mut app = Application::new(instance, doc);
        app.install_workspace_checkpoint(Some(Profile::open(files.root()).unwrap()), true)
            .unwrap();
        app.projection_sender
            .start(files.root().to_owned(), app.execution_wake.callback());
        app.projection_sender.profile.collect(true);
        assert_eq!(
            journal(&files)["entries"][0]["pending"]["request"],
            original
        );
        app.authorization.borrow_mut().origin = Some(origin.clone());
        app.authorization.borrow_mut().consent.install_paused(true);
        app.authorization_owner
            .start(Task::Configured {
                config: config(&origin, files.root()).unwrap(),
                action: crate::device_authorization_config::Action::Renew,
            })
            .unwrap();
        let handle = app.authorization_owner.wait_handles().next().unwrap();
        assert_eq!(
            WaitForSingleObject(HANDLE(handle.as_raw_handle()), 5000),
            WAIT_OBJECT_0
        );
        app.process_authorization_with_projection(|o| config(o, files.root()));
        finish(&mut app, &files);
        let calls = peer.trace.lock().unwrap().len();
        assert_eq!(snapshot(&app).document().mode(), edit::Mode::OneWay);
        command(&mut app, &files, ui::editing::TWO_WAY);
        finish(&mut app, &files);
        assert_eq!(
            peer.trace.lock().unwrap().len(),
            calls,
            "new mode may not replace unknown pending"
        );
        assert_eq!(
            journal(&files)["entries"][0]["pending"]["request"],
            original
        );
        command(&mut app, &files, ui::editing::SUBMIT);
        finish(&mut app, &files);
        assert_eq!(snapshot(&app).document().mode(), edit::Mode::TwoWay);
        assert!(journal(&files)["entries"][0]["pending"].is_null());
        let trace = peer.trace.lock().unwrap();
        let modes: Vec<_> = trace
            .iter()
            .filter(|r| r["body"]["operation"] == "mode")
            .collect();
        assert_eq!(modes.len(), 2);
        assert!(modes.iter().all(|row| row["body"] == original));
        drop(trace);
        app.shutdown();
    }
}

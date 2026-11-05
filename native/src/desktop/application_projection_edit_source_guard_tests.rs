//! 真实Application拒绝/丢回复/取消/换源/期间编辑；所有失败保原PNG、stage和精确pending。
use super::*;
use std::sync::atomic::Ordering;
unsafe fn sent(
    app: &mut Application,
    files: &Files,
    identity: crate::device_identity_store::Store,
    client: crate::device_session_client::Client,
) {
    let origin = client.origin().to_owned();
    groups::authorize(app, files, identity, client);
    app.authorization.borrow_mut().targets.directory = Some(groups::directory(&origin, 1));
    mark(app);
    send(app, files, 0);
}
#[test]
fn source_background_sync_keeps_frozen_png_and_caches_only_current_content_identity() {
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
        let unit = app.documents[0].state.unit_id.clone();
        let id = snapshot(&app).projection_id().to_owned();
        // 普通响应缺editing仍须从只读probe恢复，不重发initialize/attach。
        peer.omit_editing.store(true, Ordering::Release);
        for _ in 0..2 {
            app.process_authorization_with_projection(|o| config(o, files.root()));
            app.projection_sender.updates.make_due();
            app.process_authorization_with_projection(|o| config(o, files.root()));
            assert!(app.projection_edit.busy());
            finish(&mut app, &files);
            assert!(app.projection_source_cached(&unit, &id));
        }
        assert!(app.documents[0]
            .state
            .canvas
            .borrow_mut()
            .editor
            .translate(0, Point { x: 1.0, y: 0.0 })
            .unwrap());
        assert!(!app.projection_source_cached(&unit, &id));
        assert_eq!(
            peer.trace
                .lock()
                .unwrap()
                .iter()
                .filter(|row| row["path"] == "/v1/projections/update")
                .count(),
            1
        );
        assert_eq!(
            peer.trace
                .lock()
                .unwrap()
                .iter()
                .filter(|row| row["body"]["operation"] == "initialize")
                .count(),
            1
        );
        peer.omit_editing.store(false, Ordering::Release);
        // 缺本地Source owner时，普通sender也必须拒绝远端editing，不能烘焙覆盖其basis。
        app.documents[0].state.projection_edit.borrow_mut().take();
        let before = app.restored_session.as_ref().unwrap().original.clone();
        app.projection_sender.updates.make_due();
        app.process_authorization_with_projection(|o| config(o, files.root()));
        assert!(matches!(app.projection_sender.phase, Phase::Updating(_)));
        wait_remote(&app);
        app.process_authorization_with_projection(|o| config(o, files.root()));
        assert!(matches!(app.projection_sender.phase, Phase::Idle));
        assert!(before.same_storage(&app.restored_session.as_ref().unwrap().original));
        assert_eq!(
            peer.trace
                .lock()
                .unwrap()
                .iter()
                .filter(|row| row["path"] == "/v1/projections/update")
                .count(),
            1
        );
        assert!(!app.projection_source_targets(&unit, &origin).is_empty());
        app.shutdown();
    }
}
#[test]
fn source_active_mosaic_is_rejected_before_stage_or_network_without_changing_editor() {
    let _serial = serial();
    let _com = Apartment::new().unwrap();
    let (files, identity, _) = Files::new();
    let (listener, client, tls, pem) =
        crate::device_session_client::tests::fixture_with_host("127.0.0.1");
    std::fs::write(files.root().join("ca.pem"), pem).unwrap();
    let peer = peer::Peer::start(listener, tls, files.root().to_path_buf());
    unsafe {
        let (_class, mut app) = application(files.root());
        sent(&mut app, &files, identity, client);
        let mut canvas = app.documents[0].state.canvas.borrow_mut();
        canvas
            .editor
            .begin(Tool::Mosaic, Point { x: 20.0, y: 20.0 })
            .unwrap();
        assert!(canvas.editor.commit());
        let before = canvas.editor.document_stamp();
        drop(canvas);
        let calls = peer.trace.lock().unwrap().len();
        begin(&mut app, &files);
        completed(&app);
        poll(&mut app, &files);
        assert!(!app.projection_edit.busy());
        assert!(app.documents[0].state.projection_edit.borrow().is_none());
        assert!(before.matches(&app.documents[0].state.canvas.borrow().editor));
        assert_eq!(peer.trace.lock().unwrap().len(), calls);
        assert!(!files.root().join("projection-edit.json").exists());
        app.shutdown();
    }
}
#[test]
fn source_unsupported_probe_does_not_replace_png_or_initialize_or_modify_editor() {
    let _serial = serial();
    let _com = Apartment::new().unwrap();
    for status in [503, 501, 403] {
        let (files, identity, _) = Files::new();
        let (listener, client, tls, pem) =
            crate::device_session_client::tests::fixture_with_host("127.0.0.1");
        std::fs::write(files.root().join("ca.pem"), pem).unwrap();
        let peer = peer::Peer::start(listener, tls, files.root().to_path_buf());
        peer.probe_status.store(status, Ordering::Release);
        unsafe {
            let (_class, mut app) = application(files.root());
            sent(&mut app, &files, identity, client);
            let original = app.restored_session.as_ref().unwrap().original.clone();
            let stamp = app.documents[0]
                .state
                .canvas
                .borrow()
                .editor
                .document_stamp();
            begin(&mut app, &files);
            completed(&app);
            poll(&mut app, &files);
            assert!(!app.projection_edit.busy());
            assert!(app.documents[0].state.projection_edit.borrow().is_none());
            assert!(original.same_storage(&app.restored_session.as_ref().unwrap().original));
            assert!(stamp.matches(&app.documents[0].state.canvas.borrow().editor));
            assert!(journal(&files)["entries"][0]["source"].is_object());
            assert!(journal(&files)["entries"][0]["pending"].is_null());
            let trace = peer.trace.lock().unwrap();
            assert!(!trace
                .iter()
                .any(|e| e["path"] == "/v1/projections/update"
                    || e["body"]["operation"] == "initialize"));
            drop(trace);
            app.shutdown();
        }
    }
}
#[test]
fn source_network_period_edit_to_existing_object_is_not_mistaken_for_staged_baseline() {
    let _serial = serial();
    let _com = Apartment::new().unwrap();
    let (files, identity, _) = Files::new();
    let (listener, client, tls, pem) =
        crate::device_session_client::tests::fixture_with_host("127.0.0.1");
    std::fs::write(files.root().join("ca.pem"), pem).unwrap();
    let _peer = peer::Peer::start(listener, tls, files.root().to_path_buf());
    unsafe {
        let (_class, mut app) = application(files.root());
        sent(&mut app, &files, identity, client);
        begin(&mut app, &files);
        completed(&app);
        assert!(app.documents[0]
            .state
            .canvas
            .borrow_mut()
            .editor
            .translate(0, Point { x: 3.0, y: 0.0 })
            .unwrap());
        poll(&mut app, &files);
        let state = snapshot(&app);
        let id = state.document().objects().keys().next().unwrap();
        assert_eq!(state.document().objects()[id].value()["x"], 4.0);
        assert_eq!(state.editor_view().unwrap().basis_objects()[id]["x"], 7.0);
        assert!(state.conflict().is_none());
        assert!(journal(&files)["entries"][0]["pending"].is_object());
        finish(&mut app, &files);
        assert!(
            journal(&files)["entries"][0]["pending"].is_null(),
            "{}",
            app.authorization.borrow().editing.status
        );
        assert_eq!(
            snapshot(&app).editor_view().unwrap().basis_objects()[id]["x"],
            7.0
        );
        app.shutdown();
    }
}
#[test]
fn source_unknown_initialize_result_survives_real_reopen_and_retries_identical_initial_request() {
    let _serial = serial();
    let _com = Apartment::new().unwrap();
    let (files, identity, _) = Files::new();
    let (listener, client, tls, pem) =
        crate::device_session_client::tests::fixture_with_host("127.0.0.1");
    let origin = client.origin().to_owned();
    std::fs::write(files.root().join("ca.pem"), pem).unwrap();
    let peer = peer::Peer::start(listener, tls, files.root().to_path_buf());
    peer.lose_initialize.store(true, Ordering::Release);
    unsafe {
        let (_class, mut app) = application(files.root());
        sent(&mut app, &files, identity, client);
        begin(&mut app, &files);
        completed(&app);
        poll(&mut app, &files);
        assert!(!app.projection_edit.busy());
        assert!(app.documents[0].state.projection_edit.borrow().is_none());
        let original = journal(&files)["entries"][0]["pending"]["request"].clone();
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
        assert!(!app.projection_edit.busy());
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
        begin(&mut app, &files);
        finish(&mut app, &files);
        assert!(
            journal(&files)["entries"][0]["pending"].is_null(),
            "{}",
            app.authorization.borrow().editing.status
        );
        let trace = peer.trace.lock().unwrap();
        let calls: Vec<_> = trace
            .iter()
            .filter(|e| e["body"]["operation"] == "initialize")
            .collect();
        assert_eq!(calls.len(), 2);
        assert_eq!(calls[0]["body"], calls[1]["body"]);
        assert_eq!(
            trace
                .iter()
                .filter(|e| e["path"] == "/v1/projections/update")
                .count(),
            1
        );
        drop(trace);
        app.shutdown();
    }
}
#[test]
fn source_cancel_or_replaced_pixels_after_reply_does_not_publish_late_session() {
    let _serial = serial();
    let _com = Apartment::new().unwrap();
    for replace in [false, true] {
        let (files, identity, _) = Files::new();
        let (listener, client, tls, pem) =
            crate::device_session_client::tests::fixture_with_host("127.0.0.1");
        std::fs::write(files.root().join("ca.pem"), pem).unwrap();
        let peer = peer::Peer::start(listener, tls, files.root().to_path_buf());
        unsafe {
            let (_class, mut app) = application(files.root());
            sent(&mut app, &files, identity, client);
            begin(&mut app, &files);
            completed(&app);
            if replace {
                app.documents[0]
                    .state
                    .canvas
                    .borrow_mut()
                    .replace_image(Image {
                        width: 32,
                        height: 32,
                        pixels: [0, 255, 0, 255].repeat(1024).into(),
                    })
                    .unwrap();
            } else {
                ui::execute(
                    app.documents[0].window.0,
                    &app.documents[0].state,
                    ui::editing::CANCEL,
                )
                .unwrap();
                app.projection_edit_requests(&mut |o| config(o, files.root()));
            }
            poll(&mut app, &files);
            assert!(!app.projection_edit.busy());
            assert!(app.documents[0].state.projection_edit.borrow().is_none());
            assert!(journal(&files)["entries"][0]["pending"].is_object());
            let calls = peer.trace.lock().unwrap().len();
            if replace {
                begin(&mut app, &files);
                completed(&app);
                poll(&mut app, &files);
                assert!(!app.projection_edit.busy());
                assert_eq!(peer.trace.lock().unwrap().len(), calls);
            }
            app.shutdown();
        }
    }
}

#[test]
fn source_checkpoint_transition_rejects_equal_bytes_new_allocation_cancel_and_revocation() {
    let _serial = serial();
    let _com = Apartment::new().unwrap();
    for case in 0..3 {
        let (files, identity, _) = Files::new();
        let (listener, client, tls, pem) =
            crate::device_session_client::tests::fixture_with_host("127.0.0.1");
        let origin = client.origin().to_owned();
        std::fs::write(files.root().join("ca.pem"), pem).unwrap();
        let _peer = peer::Peer::start(listener, tls, files.root().to_path_buf());
        unsafe {
            let (_class, mut app) = application(files.root());
            sent(&mut app, &files, identity, client);
            begin(&mut app, &files);
            completed(&app);
            poll(&mut app, &files);
            let before = snapshot(&app);
            let request = journal(&files)["entries"][0]["pending"]["request"].clone();
            assert!(request.is_object());
            assert!(app.projection_edit.wait_handle().is_none());
            match case {
                0 => {
                    let record = app.restored_session.as_mut().unwrap();
                    let copy = crate::session_source::Source::from(record.original.to_vec());
                    assert!(!record.original.same_storage(&copy));
                    record.edit_original(copy);
                }
                1 => {
                    ui::execute(
                        app.documents[0].window.0,
                        &app.documents[0].state,
                        ui::editing::CANCEL,
                    )
                    .unwrap();
                    app.projection_edit_requests(&mut |o| config(o, files.root()));
                }
                _ => app.authorization_owner.invalidate(&origin).unwrap(),
            }
            poll(&mut app, &files);
            assert!(!app.projection_edit.busy(), "case {case}");
            assert_eq!(snapshot(&app), before, "case {case}");
            // 之后保存也不能追认已失效的过渡态owner；原请求保留给显式恢复。
            settle(&mut app);
            poll(&mut app, &files);
            assert_eq!(
                journal(&files)["entries"][0]["pending"]["request"],
                request,
                "case {case}"
            );
            app.shutdown();
        }
    }
}

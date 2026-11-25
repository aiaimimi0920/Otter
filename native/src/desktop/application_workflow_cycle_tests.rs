//! Complete success path through real hidden HWNDs, actual archive replacement, and pinned-certificate TLS/WS, isolated from user services.
use super::*;
use crate::desktop::{document_window::DocumentWindow, Apartment};
use std::{
    sync::atomic::AtomicBool,
    time::{Duration, Instant},
};
#[test]
fn shared_workflow_observation_checks_both_geometry_baselines_and_transient_state() {
    let _apartment = Apartment::new().unwrap();
    unsafe {
        let instance = crate::desktop::workspace_window_fixture::register();
        let document =
            DocumentWindow::create(instance, State::new(None, false).unwrap(), false).unwrap();
        let app = Application::new(instance, document);
        let hwnd = app.documents[0].window.0;
        let mut origin = RECT::default();
        GetWindowRect(hwnd, &mut origin).unwrap();
        let move_to = |x| {
            SetWindowPos(
                hwnd,
                None,
                x,
                origin.top,
                0,
                0,
                SWP_NOZORDER | SWP_NOSIZE | SWP_NOACTIVATE,
            )
            .unwrap();
        };
        let saved = save_state::Stamp::capture(&app).unwrap();
        let guard = guard::Guard::capture(&app).unwrap();
        assert!(guard.matches_saved(&app, &saved));
        move_to(origin.left + 37);
        let moved_saved = save_state::Stamp::capture(&app).unwrap();
        let moved_guard = guard::Guard::capture(&app).unwrap();
        assert!(moved_guard.matches_saved(&app, &moved_saved));
        // Saved-state validation passes, but old restore credentials must still reject because the same HWND moved.
        assert!(moved_saved.matches(&app));
        assert!(!guard.matches_saved(&app, &moved_saved));
        // New restore credentials cannot replace the old save baseline.
        assert!(moved_guard.matches(&app));
        assert!(!moved_guard.matches_saved(&app, &saved));
        move_to(origin.left);
        assert!(guard.matches_saved(&app, &saved));
        let state = &app.documents[0].state;
        state.new_window.set(true);
        assert!(
            saved.matches(&app),
            "transient request does not dirty the save"
        );
        assert!(!guard.matches_saved(&app, &saved));
        assert!(
            state.new_window.get(),
            "matching must not consume the request"
        );
        state.new_window.set(false);
        assert!(guard.matches_saved(&app, &saved));
    }
}

#[test]
fn real_application_cycle_archives_before_sending_committed_revision() {
    let _apartment = Apartment::new().unwrap();
    unsafe {
        let instance = crate::desktop::workspace_window_fixture::register();
        let image = crate::desktop::image::Image {
            width: 2,
            height: 1,
            pixels: vec![10, 20, 30, 255, 40, 50, 60, 255].into(),
        };
        let document =
            DocumentWindow::create(instance, State::new(Some(image), false).unwrap(), false)
                .unwrap();
        let mut app = Application::new(instance, document);
        let directory = std::env::temp_dir().join(format!(
            "otter-cycle-success-{}",
            crate::desktop::document_state::new_unit_id().unwrap()
        ));
        std::fs::create_dir(&directory).unwrap();
        let images = directory.join("images");
        std::fs::create_dir(&images).unwrap();
        let root = Arc::new(Root::open(&images).unwrap());
        let path = directory.join("session.json");
        app.request_save_new(path.clone()).unwrap();
        let deadline = Instant::now() + Duration::from_secs(10);
        let prepared = loop {
            if let Some(prepared) = app.take_save_candidate().unwrap() {
                break prepared;
            }
            assert!(Instant::now() < deadline, "Save worker did not finish");
            std::thread::sleep(Duration::from_millis(1));
        };
        prepared
            .commit_ready(&mut app, &AtomicBool::new(false))
            .unwrap();
        assert_eq!(app.save_status(), save_state::Status::Unchanged);
        let guard = guard::Guard::capture(&app).unwrap();
        assert!(guard.matches(&app));
        app.documents[0].state.new_window.set(true);
        assert!(!guard::is_idle(&app));
        assert!(!guard.matches(&app));
        assert!(app.documents[0].state.new_window.get());
        app.documents[0].state.new_window.set(false);
        assert!(guard.matches(&app));
        let unit = app.documents[0].state.unit_id.clone();
        let group = crate::workspace_id::UnitId::parse("fixture-group").unwrap();
        app.workspace
            .upsert_group(crate::workspace::Group {
                id: group.clone(),
                name: "Fixture".into(),
                hidden: false,
                locked: false,
            })
            .unwrap();
        assert!(!guard.matches(&app));
        let grouped = guard::Guard::capture(&app).unwrap();
        app.workspace
            .assign_group(std::slice::from_ref(&unit), Some(&group))
            .unwrap();
        assert!(!grouped.matches(&app));
        app.workspace.assign_group(&[unit], None).unwrap();
        assert!(grouped.matches(&app));
        assert!(app.workspace.remove_group(&group));
        assert!(guard.matches(&app));
        let baseline = app.restored_session.as_ref().unwrap().original.clone();
        let revision = crate::session_document::decode(&baseline).unwrap().revision;
        let (listener, identity, config) = crate::loom_bridge_client::tests::fixture();
        listener.set_nonblocking(true).unwrap();
        let handshake = crate::loom_handshake::response(
            &serde_json::to_vec(&crate::loom_handshake::tests::fixture()).unwrap(),
        )
        .unwrap()
        .unwrap();
        app.catalog.borrow_mut().loaded = Some(Rc::new(crate::desktop::capability_job::Loaded {
            identity,
            handshake,
        }));
        let saved_path = path.clone();
        let server = std::thread::spawn(move || {
            let deadline = Instant::now() + Duration::from_secs(10);
            let tcp = loop {
                match listener.accept() {
                    Ok((stream, _)) => break stream,
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        assert!(Instant::now() < deadline, "No workflow connection arrived");
                        std::thread::sleep(Duration::from_millis(1));
                    }
                    Err(error) => panic!("Workflow listener failed: {error}"),
                }
            };
            let stream = crate::loom_bridge_stream::BridgeStream::new(
                tcp,
                Instant::now() + Duration::from_secs(5),
            )
            .unwrap();
            let mut socket = tungstenite::accept(rustls::StreamOwned::new(
                rustls::ServerConnection::new(config).unwrap(),
                stream,
            ))
            .unwrap();
            let message = socket.read().unwrap();
            let request: serde_json::Value =
                serde_json::from_str(message.to_text().unwrap()).unwrap();
            assert_eq!(request["method"], "loom.hook.workflow.sync");
            assert_eq!(request["params"]["workflowId"], "hook-live");
            let snapshot = &request["params"]["snapshot"];
            assert_eq!(snapshot["documentRevision"], revision + 1);
            assert_eq!(snapshot["nodes"].as_array().unwrap().len(), 1);
            // Inspect disk before the receipt to prove network messages come from the newly durably committed baseline.
            let disk: serde_json::Value =
                serde_json::from_slice(&std::fs::read(saved_path).unwrap()).unwrap();
            assert_eq!(disk["documentRevision"], revision + 1);
            let id = disk["stickers"][0]["id"].as_str().unwrap();
            let source = disk["stickers"][0]["src"].as_str().unwrap();
            assert!(std::path::Path::new(source).is_file());
            assert_eq!(
                disk["workflowAssetArchiveIndex"]["workflows"]["hook-live"]["nodes"][id]["src"],
                source
            );
            socket.send(tungstenite::Message::Text(serde_json::json!({"protocolVersion":"loom.hook.v1","requestId":request["params"]["requestId"],"status":"succeeded","data":{"workflowId":"hook-live"}}).to_string())).unwrap();
        });
        app.request_workflow_cycle(root.clone(), 123).unwrap();
        let deadline = Instant::now() + Duration::from_secs(12);
        let outcome = loop {
            app.process_workflow_sync();
            if let Some(outcome) = app.take_workflow_cycle() {
                break outcome;
            }
            assert!(Instant::now() < deadline, "Workflow cycle did not finish");
            std::thread::sleep(Duration::from_millis(1));
        };
        server.join().unwrap();
        assert!(outcome.current);
        let report = outcome.sync.unwrap();
        assert_eq!(report.acknowledged, 1);
        assert_eq!(report.failure, None);
        assert!(outcome.archive.unwrap().retained_recovery.is_none());
        assert!(app.take_workflow_cycle().is_none());
        assert!(app
            .restored_session
            .as_ref()
            .unwrap()
            .workflow_delivery
            .is_none());
        assert_eq!(app.save_status(), save_state::Status::Unchanged);
        assert!(!app
            .restored_session
            .as_ref()
            .unwrap()
            .original
            .same_storage(&baseline));
        assert_eq!(
            std::fs::read(&path).unwrap(),
            app.restored_session.as_ref().unwrap().original.as_ref()
        );
        drop((app, root));
        std::fs::remove_dir_all(directory).unwrap();
    }
}

#[test]
fn workspace_publication_does_not_wait_for_workflow_handshake_or_receipt() {
    for after_send in [false, true] {
        unsafe { verify_workflow_retirement(after_send) };
    }
}

unsafe fn verify_workflow_retirement(after_send: bool) {
    use crate::desktop::application::restore_batch::smoke::prepared;
    use std::io::{ErrorKind, Read};
    use std::sync::mpsc;

    let _apartment = Apartment::new().unwrap();
    let instance = crate::desktop::workspace_window_fixture::register();
    let image = crate::desktop::image::Image {
        width: 1,
        height: 1,
        pixels: vec![10, 20, 30, 255].into(),
    };
    let document =
        DocumentWindow::create(instance, State::new(Some(image), false).unwrap(), false).unwrap();
    let mut app = Application::new(instance, document);
    let previous_hwnd = app.documents[0].window.0;
    let path = std::env::temp_dir().join(format!(
        "otter-retire-workflow-{}.json",
        crate::desktop::document_state::new_unit_id().unwrap()
    ));
    struct SavedFile(std::path::PathBuf);
    impl Drop for SavedFile {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.0);
        }
    }
    let _saved_file = SavedFile(path.clone());
    let cancel = AtomicBool::new(false);
    app.request_save_new(path).unwrap();
    wait_for(|| app.take_save_candidate().unwrap())
        .commit_ready(&mut app, &cancel)
        .unwrap();
    let baseline = app.restored_session.as_ref().unwrap().original.clone();
    let (listener, identity, config) = crate::loom_bridge_client::tests::fixture();
    listener.set_nonblocking(true).unwrap();
    let handshake = crate::loom_handshake::response(
        &serde_json::to_vec(&crate::loom_handshake::tests::fixture()).unwrap(),
    )
    .unwrap()
    .unwrap();
    let catalog = Rc::new(crate::desktop::capability_job::Loaded {
        identity,
        handshake,
    });
    app.catalog.borrow_mut().loaded = Some(catalog.clone());
    let frame = crate::pin_geometry::PinRect {
        x: 50,
        y: 50,
        width: 100,
        height: 100,
    };
    let batch = app
        .prepare_restored_batch(prepared(&catalog).unwrap(), &cancel, |_| Ok(frame))
        .unwrap();
    app.request_workflow_preparation(None).unwrap();
    let candidate = wait_for(|| app.take_workflow_preparation().unwrap());

    let (phase_tx, phase_rx) = mpsc::sync_channel(1);
    let (release_tx, release_rx) = mpsc::sync_channel::<()>(1);
    let peer = std::thread::spawn(move || {
        let mut tcp = wait_for(|| match listener.accept() {
            Ok((stream, _)) => Some(stream),
            Err(error) if error.kind() == ErrorKind::WouldBlock => None,
            Err(error) => panic!("Loopback listener failed: {error}"),
        });
        if after_send {
            let stream = crate::loom_bridge_stream::BridgeStream::new(
                tcp,
                Instant::now() + Duration::from_secs(6),
            )
            .unwrap();
            let mut socket = tungstenite::accept(rustls::StreamOwned::new(
                rustls::ServerConnection::new(config).unwrap(),
                stream,
            ))
            .unwrap();
            let request: serde_json::Value =
                serde_json::from_str(socket.read().unwrap().to_text().unwrap()).unwrap();
            assert_eq!(request["method"], "loom.hook.workflow.sync");
            phase_tx.send(()).unwrap();
            // Do not answer. Closure proves the client released its transport.
            assert!(matches!(
                socket.read(),
                Err(tungstenite::Error::Protocol(
                    tungstenite::error::ProtocolError::ResetWithoutClosingHandshake
                )) | Err(tungstenite::Error::Io(_))
            ));
        } else {
            // Confirm TLS started while leaving its response gated until publication returns.
            tcp.set_nonblocking(false).unwrap();
            tcp.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
            tcp.read_exact(&mut [0]).unwrap();
            phase_tx.send(()).unwrap();
            release_rx.recv_timeout(Duration::from_secs(6)).unwrap();
        }
        assert!(matches!(listener.accept(), Err(error) if error.kind() == ErrorKind::WouldBlock));
    });
    app.request_workflow_sync(candidate).unwrap();
    wait_for(|| match phase_rx.try_recv() {
        Ok(()) => Some(()),
        Err(mpsc::TryRecvError::Empty) => {
            app.process_workflow_sync();
            None
        }
        Err(error) => panic!("Peer did not reach sync phase: {error}"),
    });
    let started = Instant::now();
    app.publish_restored_batch(batch, false).unwrap();
    if !after_send {
        // The old implementation joined the deliberately stalled TLS worker here for ~3 s.
        assert!(
            started.elapsed() < Duration::from_secs(2),
            "Restore waited for TLS"
        );
    }
    assert!(!IsWindow(Some(previous_hwnd)).as_bool());
    let record = app.restored_session.as_ref().unwrap();
    assert!(!record.original.same_storage(&baseline));
    let replacement = record.original.clone();
    assert!(app.retired_workflow.is_some());
    let blocked = app.request_workflow_preparation(None).unwrap_err();
    assert!(blocked.message().contains("Previous workspace workflow"));
    if !after_send {
        release_tx.send(()).unwrap();
    }
    wait_for(|| {
        app.process_workflow_sync();
        app.retired_workflow.is_none().then_some(())
    });
    peer.join().unwrap();
    assert!(app.require_workflow_slot().is_ok());
    assert!(app
        .restored_session
        .as_ref()
        .unwrap()
        .original
        .same_storage(&replacement));
    assert!(app.take_workflow_delivery().is_none());
    assert!(app.take_workflow_cycle().is_none());
}

fn wait_for<T>(mut step: impl FnMut() -> Option<T>) -> T {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if let Some(value) = step() {
            return value;
        }
        assert!(
            Instant::now() < deadline,
            "Workflow fixture exceeded deadline"
        );
        std::thread::sleep(Duration::from_millis(1));
    }
}

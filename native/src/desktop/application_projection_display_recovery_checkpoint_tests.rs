//! 原始像素/逻辑投影、manual save 新 Source 与非 canonical 恢复副本边界。
use super::*;

#[test]
fn projection_display_recovery_manual_save_requires_a_new_canonical_commit_before_read() {
    let _serial = crate::loom_bridge_client::tests::execution_job::SERIAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let _com = Apartment::new().unwrap();
    unsafe {
        let mut fixture = Fixture::new();
        saved(&mut fixture);
        let entry = fixture.app.projection_profile.entries().unwrap()[0].clone();
        let path = fixture.files.root().join("manual-recovery.json");
        fixture.app.request_save_new(path.clone()).unwrap();
        let end = Instant::now() + Duration::from_secs(10);
        let prepared = loop {
            if let Some(candidate) = fixture.app.take_save_candidate().unwrap() {
                break candidate;
            }
            assert!(
                Instant::now() < end,
                "display recovery manual candidate deadline"
            );
            std::thread::yield_now();
        };
        prepared
            .commit_ready(&mut fixture.app, &std::sync::atomic::AtomicBool::new(false))
            .unwrap();
        assert!(path.is_file());
        fixture
            .app
            .start_projection_display_recovery(entry.clone(), &mut |_| {
                panic!("manual file became canonical")
            });
        assert!(!fixture.app.projection_receive.busy());
        fixture
            .app
            .start_checkpoint_with(&mut |_, _, message, warning| assert!(!warning, "{message}"));
        checkpoint(&mut fixture.app);
        poll(&mut fixture);
        peer(&mut fixture, 0, false);
        choose(&mut fixture);
        settle(&mut fixture);
        joined(&mut fixture);
        assert_eq!(
            fixture.app.authorization.borrow().projection.status,
            "投送已呈现"
        );
    }
}
#[test]
fn projection_display_recovery_recovery_copy_reopen_never_grants_canonical_permission() {
    let _serial = crate::loom_bridge_client::tests::execution_job::SERIAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let _com = Apartment::new().unwrap();
    unsafe {
        let mut fixture = Fixture::new();
        saved(&mut fixture);
        let journal = fixture.journal();
        fixture.app.shutdown();
        let path = fixture.files.root().join("workspace.json");
        let recovery = fixture
            .files
            .root()
            .join("workspace.json.otter-recovery-01234567-89AB-CDEF-0123-456789ABCDEF.json");
        std::fs::rename(&path, &recovery).unwrap();
        fixture::reopen_workspace(&mut fixture);
        let entry = fixture.app.projection_profile.entries().unwrap()[0].clone();
        assert!(fixture.unit_present());
        assert!(fixture
            .app
            .projection_checkpoint_inclusion(&entry)
            .is_none());
        fixture
            .app
            .start_projection_display_recovery(entry, &mut |_| {
                panic!("recovery copy reached config")
            });
        assert!(!fixture.app.projection_receive.busy());
        assert!(!path.exists() && recovery.exists());
        assert_eq!(fixture.journal(), journal);
    }
}
#[test]
fn projection_display_recovery_reporting_rechecks_other_saved_document_changes() {
    let _serial = crate::loom_bridge_client::tests::execution_job::SERIAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let _com = Apartment::new().unwrap();
    unsafe {
        let mut fixture = Fixture::new();
        saved(&mut fixture);
        let journal = fixture.journal();
        let listener = fixture.listener.take().unwrap();
        let tls = fixture.tls.clone();
        let origin = fixture.origin.clone();
        let (tx, rx) = mpsc::sync_channel(1);
        fixture.server = Some(std::thread::spawn(move || {
            let mut stream = StreamOwned::new(
                ServerConnection::new(tls.clone()).unwrap(),
                accept(&listener),
            );
            let (head, _) = request(&mut stream);
            assert!(head.starts_with("POST /v1/projections/read "));
            reply(
                &mut stream,
                200,
                &serde_json::to_vec(&fixture::response(&origin)).unwrap(),
            );
            drop(stream);
            let mut stream =
                StreamOwned::new(ServerConnection::new(tls).unwrap(), accept(&listener));
            let (head, _) = request(&mut stream);
            assert!(head.starts_with("POST /v1/projections/receipt "));
            tx.send(()).unwrap();
            assert!(!stream.read(&mut [0]).is_ok_and(|n| n != 0));
            listener
        }));
        choose(&mut fixture);
        let end = Instant::now() + Duration::from_secs(10);
        while rx.try_recv().is_err() {
            poll(&mut fixture);
            assert!(Instant::now() < end, "reporting strict proof deadline");
            std::thread::yield_now();
        }
        let host = fixture
            .app
            .documents
            .iter()
            .find(|doc| doc.state.unit_id.as_str() != fixture.ticket().unit())
            .unwrap();
        host.state.canvas.borrow_mut().normal_opacity = 0.5;
        settle(&mut fixture);
        joined(&mut fixture);
        assert_eq!(fixture.journal(), journal);
        assert_ne!(
            fixture.app.authorization.borrow().projection.status,
            "投送已呈现"
        );
        assert!(!fixture.app.projection_cleanup.pending_local());
    }
}

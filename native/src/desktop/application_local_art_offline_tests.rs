//! 真实OpenArt在线预热后关闭TLS服务，重建无Device会话的Application离线执行并保存恢复。
use super::*;
const SOURCE: &str =
    "NeuroSurface.define({mount(){NeuroSurface.commitResult({answer:{kind:'value',value:42}})}});";
#[test]
fn reopened_application_without_device_session_runs_cached_authorized_art_and_saves_result() {
    let _javascript = crate::surface_javascript::fixture::SERIAL.lock().unwrap();
    unsafe {
        let _environment = environment();
        let peer = peer::Peer::with_source(SOURCE, true);
        let config = peer.config();
        let origin = peer.origin.clone();
        let trust = peer.authority();
        let mut first = image_app();
        peer.authorize(&mut first.authorization_owner);
        first.art_issuers = peer.authority();
        let from = first.documents[0].state.unit_id.to_string();
        first
            .open_local_art(&origin, "vendor/application".into(), &from, peer.config())
            .unwrap();
        settle_local(&mut first, (1, 1));
        first.shutdown();
        peer.finish();
        let mut app = image_app();
        app.art_issuers = trust;
        assert!(app
            .authorization_owner
            .lease(&origin, crate::desktop::surface_windows::now_ms().unwrap())
            .is_err());
        let from = app.documents[0].state.unit_id.to_string();
        app.open_local_art(&origin, "vendor/application".into(), &from, config)
            .unwrap();
        let target = app.documents[1].state.unit_id.clone();
        settle_local(&mut app, (1, 1));
        app.settle_live_viewers().unwrap();
        assert!(
            app.authorization.borrow().live.bindings.is_empty(),
            "offline Art cannot supply an online viewer proof"
        );
        assert_eq!(
            app.documents[1]
                .state
                .execution_results
                .borrow()
                .save_value_outputs()
                .unwrap()["answer"],
            42
        );
        assert!(app.documents[0]
            .state
            .execution_results
            .borrow()
            .save_value_outputs()
            .is_none());
        let (_, snapshot) = app.capture_session_snapshot().unwrap();
        let cancel = std::sync::atomic::AtomicBool::new(false);
        let encoded = snapshot.encode(&cancel).unwrap();
        let candidate =
            crate::desktop::session_candidate::prepare(encoded.bytes.to_vec(), None, &cancel)
                .unwrap();
        let outputs = candidate.prepare_results(&cancel).unwrap();
        assert_eq!(
            outputs
                .iter()
                .find(|r| r.unit() == &target)
                .unwrap()
                .save_value_outputs()
                .unwrap()["answer"],
            42
        );
        app.shutdown();
        assert!(!app.local_arts.busy());
        assert!(app.authorization_owner.finished());
    }
}
#[test]
fn offline_late_prepare_cannot_publish_after_workspace_reset() {
    let _javascript = crate::surface_javascript::fixture::SERIAL.lock().unwrap();
    unsafe {
        let _environment = environment();
        let peer = peer::Peer::with_source(SOURCE, true);
        let config = peer.config();
        let origin = peer.origin.clone();
        let trust = peer.authority();
        let mut first = image_app();
        peer.authorize(&mut first.authorization_owner);
        first.art_issuers = peer.authority();
        let from = first.documents[0].state.unit_id.to_string();
        first
            .open_local_art(&origin, "vendor/application".into(), &from, peer.config())
            .unwrap();
        settle_local(&mut first, (1, 1));
        first.shutdown();
        peer.finish();
        let mut app = image_app();
        app.art_issuers = trust;
        let from = app.documents[0].state.unit_id.to_string();
        app.open_local_art(&origin, "vendor/application".into(), &from, config)
            .unwrap();
        app.local_arts.reset();
        settle_local(&mut app, (0, 0));
        assert!(app.documents.iter().all(|d| d
            .state
            .execution_results
            .borrow()
            .save_value_outputs()
            .is_none()));
        app.shutdown();
        assert!(!app.local_arts.busy());
    }
}

#[test]
fn clearing_authorization_prevents_cached_art_after_application_restart() {
    let _javascript = crate::surface_javascript::fixture::SERIAL.lock().unwrap();
    unsafe {
        let _environment = environment();
        for immediate_shutdown in [false, true] {
            let peer = peer::Peer::with_source(SOURCE, true);
            let config = peer.config();
            let origin = peer.origin.clone();
            let trust = peer.authority();
            let mut first = image_app();
            peer.authorize(&mut first.authorization_owner);
            first.art_issuers = peer.authority();
            let from = first.documents[0].state.unit_id.to_string();
            first
                .open_local_art(&origin, "vendor/application".into(), &from, peer.config())
                .unwrap();
            settle_local(&mut first, (1, 1));
            {
                let mut view = first.authorization.borrow_mut();
                view.origin = Some(origin.clone());
                view.request = Some(Request::Cancel);
            }
            first.process_authorization_with_projection(|_| Ok(peer.config()));
            if !immediate_shutdown {
                let deadline = Instant::now() + Duration::from_secs(5);
                while first.authorization.borrow().busy {
                    first.process_authorization_with_projection(|_| Ok(peer.config()));
                    assert!(Instant::now() < deadline, "offline clear deadline");
                    std::thread::sleep(Duration::from_millis(1));
                }
                assert!(first
                    .authorization
                    .borrow()
                    .status
                    .contains("离线授权已清除"));
            }
            first.shutdown();
            assert!(
                crate::art_offline_store::Slot::cutoff(config.identity_root(), &origin).unwrap()
                    > 0
            );
            peer.finish();
            let mut reopened = image_app();
            reopened.art_issuers = trust;
            let from = reopened.documents[0].state.unit_id.to_string();
            reopened
                .open_local_art(&origin, "vendor/application".into(), &from, config)
                .unwrap();
            settle_local(&mut reopened, (0, 0));
            assert!(reopened.documents.iter().all(|d| d
                .state
                .execution_results
                .borrow()
                .save_value_outputs()
                .is_none()));
            reopened.shutdown();
        }
    }
}

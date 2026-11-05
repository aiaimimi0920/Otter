//! 同一隔离profile重建Application，真实OpenArt必须取新许可并复用磁盘源码。
use super::*;
const SOURCE: &str =
    "NeuroSurface.define({mount(){NeuroSurface.commitResult({answer:{kind:'value',value:42}})}});";

#[test]
fn reopened_application_reuses_source_but_not_binding_and_commits_current_result() {
    let _javascript = crate::surface_javascript::fixture::SERIAL.lock().unwrap();
    unsafe {
        let _environment = environment();
        let peer = peer::Peer::with_cached_source(SOURCE);
        let mut owners = Vec::new();
        for _ in 0..2 {
            let mut app = image_app();
            peer.authorize(&mut app.authorization_owner);
            app.art_issuers = peer.authority();
            let source = app.documents[0].state.unit_id.clone();
            app.open_local_art(
                &peer.origin,
                "vendor/application".into(),
                source.as_str(),
                peer.config(),
            )
            .unwrap();
            let target = app.documents[1].state.unit_id.clone();
            assert!(!owners.contains(&target));
            owners.push(target.clone());
            settle_local(&mut app, (1, 1));
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
            let restored =
                crate::desktop::session_candidate::prepare(encoded.bytes.to_vec(), None, &cancel)
                    .unwrap();
            let results = restored.prepare_results(&cancel).unwrap();
            assert_eq!(
                results
                    .iter()
                    .find(|r| r.unit() == &target)
                    .unwrap()
                    .save_value_outputs()
                    .unwrap()["answer"],
                42
            );
            let cache = peer.config().identity_root().join("art-source-cache");
            let files: Vec<_> = std::fs::read_dir(cache)
                .unwrap()
                .map(|entry| entry.unwrap().path())
                .filter(|p| p.extension().is_some_and(|ext| ext == "js"))
                .collect();
            assert_eq!(files.len(), 1);
            assert_eq!(std::fs::read(&files[0]).unwrap(), SOURCE.as_bytes());
            app.shutdown();
            assert!(!app.local_arts.busy());
            assert!(app.authorization_owner.finished());
        }
        peer.finish();
    }
}

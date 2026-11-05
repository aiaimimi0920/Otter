//! 用户 OpenArt 创建独立节点、原子连接及保存恢复；失败不能遗留窗口或拓扑。
use super::*;
const SOURCE: &str =
    "NeuroSurface.define({mount(){NeuroSurface.commitResult({answer:{kind:'value',value:42}})}});";

#[test]
fn changed_input_graph_discards_queued_formal_result_but_unrelated_node_does_not() {
    let _javascript = crate::surface_javascript::fixture::SERIAL.lock().unwrap();
    unsafe {
        let _environment = environment();
        for change_input in [false, true] {
            let mut app = image_app();
            let source = app.documents[0].state.unit_id.clone();
            let peer = peer::Peer::with_source(SOURCE, true);
            peer.authorize(&mut app.authorization_owner);
            app.art_issuers = peer.authority();
            app.open_local_art(
                &peer.origin,
                "vendor/application".into(),
                source.as_str(),
                peer.config(),
            )
            .unwrap();
            let target = app.documents[1].state.unit_id.clone();
            let deadline = Instant::now() + Duration::from_secs(10);
            // 只领取运行时帧，不调用Application的结果发布；制造真实已排队的迟到结果。
            while app.local_arts.test_counts() != (1, 1) {
                app.local_arts.poll(
                    &app.documents,
                    &mut app.workspace,
                    &app.live_sessions,
                    false,
                );
                assert!(Instant::now() < deadline);
                std::thread::sleep(Duration::from_millis(1));
            }
            peer.finish();
            if change_input {
                app.workspace.retain_units(|unit| unit != &source);
            } else {
                app.workspace
                    .add_units([crate::workspace_id::UnitId::parse("unrelated").unwrap()])
                    .unwrap();
            }
            let (changed, error) =
                app.local_arts
                    .apply_results(&app.documents, &app.workspace, &app.live_sessions);
            assert!(error.is_none());
            if change_input {
                assert!(changed.is_empty());
                assert!(app.documents[1]
                    .state
                    .execution_results
                    .borrow()
                    .save_value_outputs()
                    .is_none());
                settle_local(&mut app, (0, 0));
            } else {
                assert_eq!(changed, vec![target]);
                assert_eq!(
                    app.documents[1]
                        .state
                        .execution_results
                        .borrow()
                        .save_value_outputs()
                        .unwrap()["answer"],
                    42
                );
                assert_eq!(app.local_arts.test_counts(), (1, 1));
            }
            app.shutdown();
        }
    }
}
#[test]
fn open_art_creates_independent_connected_node_and_roundtrips_formal_result() {
    let _javascript = crate::surface_javascript::fixture::SERIAL.lock().unwrap();
    unsafe {
        let _environment = environment();
        let mut app = image_app();
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
        let peer = peer::Peer::with_source(SOURCE, true);
        peer.authorize(&mut app.authorization_owner);
        let before = app.workspace.topology_stamp();
        let registered = app.capture_windows.borrow().len();
        // 缺少issuer信任：候选已准备但尚未发布，失败须完整回收。
        assert!(app
            .open_local_art(
                &peer.origin,
                "vendor/application".into(),
                source.as_str(),
                peer.config()
            )
            .is_err());
        assert!(before.matches(&app.workspace));
        assert_eq!(app.documents.len(), 1);
        assert_eq!(app.capture_windows.borrow().len(), registered);
        assert!(!app.modals.blocked());
        app.art_issuers = peer.authority();
        {
            let mut view = app.authorization.borrow_mut();
            view.closed = false;
            view.origin = Some(peer.origin.clone());
            view.request = Some(Request::OpenArt {
                art: "vendor/application".into(),
                node: source.to_string(),
            });
        }
        app.process_authorization_with_projection(|_| Ok(peer.config()));
        assert_eq!(app.authorization.borrow().status, "正在本地打开 Art…");
        assert_eq!(app.documents.len(), 2);
        assert!(!app.documents[0].state.is_art());
        let target = app.documents[1].state.unit_id.clone();
        assert_ne!(source, target);
        assert_eq!(
            app.documents[1].state.local_art_id.as_deref(),
            Some("vendor/application")
        );
        assert_eq!(app.workspace.links().len(), 1);
        let link = &app.workspace.links()[0];
        assert_eq!((&link.from_unit, &link.to_unit), (&source, &target));
        assert_eq!(
            (&*link.from_port, &*link.to_port),
            ("output", "input_image")
        );
        settle_local(&mut app, (1, 1));
        peer.finish();
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
        assert!(app.documents[0]
            .state
            .execution_results
            .borrow()
            .save_value_outputs()
            .is_none());
        assert_eq!(
            app.documents[1]
                .state
                .execution_results
                .borrow()
                .save_value_outputs()
                .unwrap()["answer"],
            42
        );
        let (_, snapshot) = app.capture_session_snapshot().unwrap();
        let cancel = std::sync::atomic::AtomicBool::new(false);
        let encoded = snapshot.encode(&cancel).unwrap();
        let restored =
            crate::desktop::session_candidate::prepare(encoded.bytes.to_vec(), None, &cancel)
                .unwrap();
        assert_eq!(restored.workspace.links(), app.workspace.links());
        let placement = restored
            .placements
            .iter()
            .find(|p| p.unit == target)
            .unwrap();
        assert!(placement.art);
        assert_eq!(
            placement.local_art_id.as_deref(),
            Some("vendor/application")
        );
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
        app.shutdown();
    }
}

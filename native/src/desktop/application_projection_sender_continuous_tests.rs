//! 真实菜单/Job/TLS/Checkpoint持续更新及迟到隔离；不占前台，不伪造接收端呈现。
use super::*;
use serde_json::{json, Value};
#[path = "application_projection_sender_update_peer.rs"]
mod peer;
pub(super) use peer::Mode;
unsafe fn cycle(app: &mut Application, root: &std::path::Path) {
    app.projection_sender.updates.make_due();
    app.process_authorization_with_projection(|origin| config(origin, root));
    assert!(matches!(app.projection_sender.phase, Phase::Updating(_)));
    wait_remote(app);
}
unsafe fn binding(app: &Application) -> Binding {
    app.workspace_checkpoint
        .as_ref()
        .unwrap()
        .sender_index()
        .unwrap()
        .entries()[0]
        .clone()
}
unsafe fn run(mode: Mode) {
    run_routed(mode, false);
}
pub(super) unsafe fn run_routed(mode: Mode, offline: bool) {
    let _serial = serial();
    let _com = Apartment::new().unwrap();
    let (files, identity, _) = Files::new();
    let (listener, client, tls, pem) =
        crate::device_session_client::tests::fixture_with_host("127.0.0.1");
    let origin = client.origin().to_owned();
    std::fs::write(files.root().join("ca.pem"), pem).unwrap();
    let (update_signal, update_received) = std::sync::mpsc::channel();
    let server = if mode == Mode::DurableStop || offline {
        peer::start_with_signal(
            listener,
            tls,
            mode,
            (mode == Mode::DurableStop).then_some(update_signal),
            offline,
        )
    } else {
        peer::start(listener, tls, mode)
    };
    let (_class, mut app) = application(files.root());
    app.authorization.borrow_mut().origin = Some(origin.clone());
    app.authorization.borrow_mut().consent.install_paused(true);
    app.authorization_owner
        .start(Task::Renew {
            client,
            store: identity,
        })
        .unwrap();
    let handle = app.authorization_owner.wait_handles().next().unwrap();
    assert_eq!(
        WaitForSingleObject(HANDLE(handle.as_raw_handle()), 5000),
        WAIT_OBJECT_0
    );
    app.process_authorization_with_projection(|origin| config(origin, files.root()));
    app.authorization.borrow_mut().targets.directory = if offline {
        Some(super::offline::directory(&origin, true))
    } else {
        Some(
            crate::projection_delivery::targets::Directory::parse(
                &serde_json::to_vec(&crate::projection_delivery::targets::tests::body()).unwrap(),
                &origin,
            )
            .unwrap()
            .into(),
        )
    };
    ui::execute(
        app.documents[0].window.0,
        &app.documents[0].state,
        ui::sender::BASE,
    )
    .unwrap();
    app.process_authorization_with_projection(|origin| config(origin, files.root()));
    wait_remote(&app);
    app.process_authorization_with_projection(|origin| config(origin, files.root()));
    settle(&mut app);
    app.process_authorization_with_projection(|origin| config(origin, files.root()));
    let original = binding(&app);
    assert_eq!(original.revision(), 1);
    assert!(original.linked());
    let outbox = files.root().join("projection-sender.json");
    let durable = std::fs::read(&outbox).unwrap();
    app.documents[0]
        .state
        .canvas
        .borrow_mut()
        .replace_image(Image {
            width: 32,
            height: 32,
            pixels: [0, 255, 0, 255].repeat(32 * 32).into(),
        })
        .unwrap();
    let before = app.restored_session.as_ref().unwrap().original.clone();
    if mode == Mode::Profile {
        app.projection_sender.updates.make_due();
        let other = files.root().join("other-profile");
        app.process_authorization_with_projection(|origin| config(origin, &other));
        assert!(!app.remote_sessions.projection_pending());
        assert!(matches!(app.projection_sender.phase, Phase::Idle));
    } else if mode == Mode::DurableStop {
        app.projection_sender.updates.make_due();
        app.process_authorization_with_projection(|origin| config(origin, files.root()));
        update_received
            .recv_timeout(Duration::from_secs(5))
            .unwrap();
        assert!(matches!(app.projection_sender.phase, Phase::Updating(_)));
        assert!(app.authorization.borrow().sender.can_stop);
        stop::run(&mut app, files.root(), &original, 6);
        drop(app);
        drain_messages();
        let listener = server.join().unwrap();
        assert!(
            matches!(listener.accept(), Err(error) if error.kind() == std::io::ErrorKind::WouldBlock)
        );
        return;
    } else {
        cycle(&mut app, files.root());
    }
    let mut prior_workspace = None;
    // worker已结束，尚未消费：模拟来源换代/用户停止，不依赖不确定的网络时序。
    if mode == Mode::Stale {
        app.documents[0]
            .state
            .canvas
            .borrow_mut()
            .replace_image(Image {
                width: 32,
                height: 32,
                pixels: [0, 0, 255, 255].repeat(32 * 32).into(),
            })
            .unwrap();
    } else if mode == Mode::Stopped {
        app.stop_projection_sender();
    } else if mode == Mode::Revoked {
        app.authorization.borrow_mut().request = Some(ui::Request::Cancel);
    } else if mode == Mode::Workspace {
        let replacement =
            crate::workspace::Workspace::with_initial(app.documents[0].state.unit_id.clone());
        prior_workspace = Some(std::mem::replace(&mut app.workspace, replacement));
    }
    app.process_authorization_with_projection(|origin| config(origin, files.root()));
    if matches!(
        mode,
        Mode::Malformed
            | Mode::Stale
            | Mode::Stopped
            | Mode::Offline
            | Mode::Revoked
            | Mode::Workspace
            | Mode::Profile
            | Mode::WrongPeer
            | Mode::PeerRevoked
    ) {
        assert!(app
            .restored_session
            .as_ref()
            .unwrap()
            .original
            .same_storage(&before));
        assert_eq!(binding(&app), original);
        assert!(matches!(app.projection_sender.phase, Phase::Idle));
        assert!(!app.projection_sender.pending_local());
        if matches!(
            mode,
            Mode::Malformed | Mode::Stale | Mode::Offline | Mode::WrongPeer | Mode::PeerRevoked
        ) {
            assert!(app.projection_sender.deadline(now().unwrap()).unwrap() > Instant::now());
        }
    } else {
        assert!(matches!(app.projection_sender.phase, Phase::Checkpoint(_)));
        settle(&mut app);
        app.process_authorization_with_projection(|origin| config(origin, files.root()));
        assert_eq!(binding(&app).revision(), 2);
        assert_ne!(binding(&app).digest(), original.digest());
        assert_eq!(binding(&app).envelope(), original.envelope());
    }
    if mode == Mode::Normal {
        // 第一次canonical变化至多多渲染一次；同PNG必须只read，不再次update。
        cycle(&mut app, files.root());
        app.process_authorization_with_projection(|origin| config(origin, files.root()));
        assert_eq!(binding(&app).revision(), 2);
        assert!(matches!(app.projection_sender.phase, Phase::Idle));
        cycle(&mut app, files.root());
        app.process_authorization_with_projection(|origin| config(origin, files.root()));
        assert!(matches!(app.projection_sender.phase, Phase::Idle));
        app.documents[0]
            .state
            .canvas
            .borrow_mut()
            .editor
            .crop_to(crate::crop_geometry::Rect {
                x: 4.,
                y: 2.,
                width: 16.,
                height: 8.,
            })
            .unwrap();
        cycle(&mut app, files.root());
        app.process_authorization_with_projection(|origin| config(origin, files.root()));
        settle(&mut app);
        app.process_authorization_with_projection(|origin| config(origin, files.root()));
        assert_eq!(binding(&app).revision(), 3);
    } else if mode == Mode::Stale {
        cycle(&mut app, files.root());
        app.process_authorization_with_projection(|origin| config(origin, files.root()));
        settle(&mut app);
        app.process_authorization_with_projection(|origin| config(origin, files.root()));
        assert_eq!(binding(&app).revision(), 3);
    }
    assert_eq!(
        std::fs::read(&outbox).unwrap(),
        durable,
        "updates/late failures cannot rewrite or cancel initial outbox"
    );
    let final_binding = binding(&app);
    if let Some(workspace) = prior_workspace {
        app.workspace = workspace;
    }
    app.shutdown();
    drop(app);
    drain_messages();
    let listener = server.join().unwrap();
    assert!(matches!(listener.accept(),Err(error) if error.kind()==std::io::ErrorKind::WouldBlock));
    // 同profile重开只从实际canonical恢复版本；没有Device Session时绝不自动更新或解绑。
    let instance = GetModuleHandleW(None).unwrap().into();
    let document =
        DocumentWindow::create(instance, State::new(None, false).unwrap(), false).unwrap();
    let mut reopened = Application::new(instance, document);
    reopened
        .install_workspace_checkpoint(Some(Profile::open(files.root()).unwrap()), true)
        .unwrap();
    assert!(reopened.projection_sender_inclusion(&final_binding));
    assert!(!reopened.remote_sessions.projection_pending());
    reopened.shutdown();
    assert_eq!(std::fs::read(&outbox).unwrap(), durable);
    let stored: Value = serde_json::from_slice(&durable).unwrap();
    assert_eq!(stored["entries"][0]["cancelled"], json!(false));
}
#[test]
fn native_sender_continuous_updates_crop_and_noop_preserve_identity_and_restore_version() {
    unsafe {
        run(Mode::Normal);
    }
}
#[test]
fn native_sender_continuous_lost_update_reply_only_reads_to_reconcile_once() {
    unsafe {
        run(Mode::Lost);
    }
}
#[test]
fn native_sender_continuous_wrong_update_digest_does_not_publish_or_cancel_stable_binding() {
    unsafe {
        run(Mode::Malformed);
    }
}
#[test]
fn native_sender_continuous_stale_finished_result_is_discarded_and_latest_content_wins() {
    unsafe {
        run(Mode::Stale);
    }
}
#[test]
fn native_sender_continuous_stop_after_completion_retains_canonical_and_outbox() {
    unsafe {
        run(Mode::Stopped);
    }
}
#[test]
fn native_sender_continuous_network_failure_backs_off_without_new_invitation_or_unlink() {
    unsafe {
        run(Mode::Offline);
    }
}
#[test]
fn native_sender_continuous_revoked_finished_result_cannot_mutate_or_unlink_stable_binding() {
    unsafe {
        run(Mode::Revoked);
    }
}
#[test]
fn native_sender_continuous_same_unit_new_workspace_identity_rejects_late_update() {
    unsafe {
        run(Mode::Workspace);
    }
}
#[test]
fn native_sender_continuous_changed_profile_cannot_use_original_outbox_identity() {
    unsafe {
        run(Mode::Profile);
    }
}
#[test]
fn native_sender_continuous_menu_stop_interrupts_inflight_update_and_commits_durable_unlink() {
    unsafe {
        run(Mode::DurableStop);
    }
}
#[test]
fn native_sender_content_cache_metadata_checkpoint_does_not_retire_original_source_scope() {
    let _serial = serial();
    let _com = Apartment::new().unwrap();
    let (files, _, _) = Files::new();
    unsafe {
        let (_class, mut app) = application(files.root());
        let unit = &app.documents[0].state.unit_id;
        let (captured, _) = scope::Scope::capture(&app, unit).unwrap();
        let (cache, _) = scope::Scope::capture(&app, unit).unwrap();
        let cache = scope::ContentStamp::from(cache);
        assert!(cache.matches(&app));
        let mut rect = RECT::default();
        GetWindowRect(app.documents[0].window.0, &mut rect).unwrap();
        // 移动自有隐藏贴图只改变持久位置；原文/像素/Editor不换代，但确实产生新canonical。
        SetWindowPos(
            app.documents[0].window.0,
            None,
            rect.left + 8,
            rect.top + 8,
            0,
            0,
            SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
        )
        .unwrap();
        settle(&mut app);
        assert!(!captured.binding_source.same_storage(
            app.workspace_checkpoint
                .as_ref()
                .unwrap()
                .sender_source()
                .unwrap()
        ));
        assert!(captured.matches(&app));
        assert!(!cache.matches(&app));
        app.shutdown();
    }
}

//! 真实组菜单→原Job/TLS→逐目标canonical；隔离profile、不注册输入、不占前台。
use super::*;
use serde_json::{json, Value};
#[path = "application_projection_sender_group_guard_tests.rs"]
mod guard;
#[path = "application_projection_sender_group_peer.rs"]
mod peer;
#[path = "application_projection_sender_stop_group_tests.rs"]
mod stopping;
pub(super) fn directory(
    origin: &str,
    count: usize,
) -> std::sync::Arc<crate::projection_delivery::targets::Directory> {
    crate::projection_delivery::targets::Directory::parse(&serde_json::to_vec(&json!({
        "targets":(0..count).map(|i| json!({"deviceId":format!("receiver-{i}"),"name":format!("Screen {i}"),"policy":"confirm","route":"shared_loom"})).collect::<Vec<_>>(),
        "groups":[{"groupId":"group:all","name":"All screens","targetIds":(0..count).map(|i| format!("receiver-{i}")).collect::<Vec<_>>(),"unavailableCount":1}]
    })).unwrap(),origin).unwrap().into()
}
pub(super) unsafe fn authorize(
    app: &mut Application,
    files: &Files,
    identity: crate::device_identity_store::Store,
    client: crate::device_session_client::Client,
) {
    let origin = client.origin().to_owned();
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
    app.process_authorization_with_projection(|o| config(o, files.root()));
}
unsafe fn start(app: &mut Application, files: &Files, origin: &str, count: usize) {
    app.authorization.borrow_mut().targets.directory = Some(directory(origin, count));
    ui::execute(
        app.documents[0].window.0,
        &app.documents[0].state,
        ui::sender::groups::BASE,
    )
    .unwrap();
    app.process_authorization_with_projection(|o| config(o, files.root()));
    assert!(matches!(app.projection_sender.phase, Phase::Waiting(_)));
}
fn run(mode: u8) {
    let _serial = serial();
    let _com = Apartment::new().unwrap();
    let (files, identity, _) = Files::new();
    let before = std::fs::read(files.root().join("device-identity.json")).unwrap();
    let (listener, client, tls, pem) = fixture_with_pem();
    let origin = client.origin().to_owned();
    std::fs::write(files.root().join("ca.pem"), pem).unwrap();
    let calls = match mode {
        0 => 6,
        1 => 7,
        2 => 6,
        3 => 5,
        _ => 2,
    };
    let server = peer::start(
        listener,
        tls,
        files.root().to_path_buf(),
        calls,
        mode == 1 || mode == 2,
    );
    unsafe {
        let (_class, mut app) = application(files.root());
        authorize(&mut app, &files, identity, client);
        start(
            &mut app,
            &files,
            &origin,
            if mode == 2 {
                3
            } else if mode == 4 {
                9
            } else {
                2
            },
        );
        let original = app.restored_session.as_ref().unwrap().original.clone();
        wait_remote(&app);
        if mode == 3 {
            app.documents[0].state.canvas.borrow_mut().normal_opacity = 0.5;
        }
        app.process_authorization_with_projection(|o| config(o, files.root()));
        if mode >= 3 {
            assert!(app.projection_sender.group.is_none());
            assert!(app
                .restored_session
                .as_ref()
                .unwrap()
                .original
                .same_storage(&original));
            if mode == 4 {
                assert!(!files.root().join("projection-sender.json").exists());
                assert!(app
                    .authorization
                    .borrow()
                    .sender
                    .group_results
                    .iter()
                    .any(|line| line.contains("失败")));
            } else {
                let end = Instant::now() + Duration::from_secs(5);
                while app.projection_sender.pending_local()
                    || app.projection_sender.cleanup.network.is_none()
                {
                    app.process_authorization_with_projection(|o| config(o, files.root()));
                    assert!(Instant::now() < end);
                    std::thread::sleep(Duration::from_millis(1));
                }
                wait_remote(&app);
                app.process_authorization_with_projection(|o| config(o, files.root()));
            }
            app.shutdown();
        } else {
            assert!(matches!(app.projection_sender.phase, Phase::Checkpoint(_)));
            let first = match &app.projection_sender.phase {
                Phase::Checkpoint(saved) => saved.expected.clone(),
                _ => unreachable!(),
            };
            assert!(!app.projection_sender_inclusion(&first));
            let journal: Value = serde_json::from_slice(
                &std::fs::read(files.root().join("projection-sender.json")).unwrap(),
            )
            .unwrap();
            assert_eq!(
                journal["entries"].as_array().unwrap().len(),
                1,
                "next target must wait for canonical"
            );
            settle(&mut app);
            app.process_authorization_with_projection(|o| config(o, files.root()));
            assert!(app.projection_sender_inclusion(&first));
            assert!(
                matches!(app.projection_sender.phase, Phase::Waiting(_)),
                "own canonical must rebase group fence"
            );
            wait_remote(&app);
            app.process_authorization_with_projection(|o| config(o, files.root()));
            if mode == 1 || mode == 2 {
                assert!(matches!(app.projection_sender.phase, Phase::Retry(_)));
                let path = files.root().join("projection-sender.json");
                let signed = std::fs::read(&path).unwrap();
                if mode == 1 {
                    ui::execute(
                        app.documents[0].window.0,
                        &app.documents[0].state,
                        ui::sender::RETRY,
                    )
                    .unwrap();
                    app.process_authorization_with_projection(|o| config(o, files.root()));
                    wait_remote(&app);
                    app.process_authorization_with_projection(|o| config(o, files.root()));
                    assert_eq!(
                        std::fs::read(path).unwrap(),
                        signed,
                        "retry keeps exact signed intent/PNG"
                    );
                } else {
                    ui::execute(
                        app.documents[0].window.0,
                        &app.documents[0].state,
                        ui::sender::CANCEL,
                    )
                    .unwrap();
                    app.process_authorization_with_projection(|o| config(o, files.root()));
                    assert!(app.projection_sender.group.is_none());
                    let end = Instant::now() + Duration::from_secs(5);
                    while app.projection_sender.pending_local()
                        || app.projection_sender.cleanup.network.is_none()
                    {
                        app.process_authorization_with_projection(|o| config(o, files.root()));
                        assert!(Instant::now() < end);
                        std::thread::sleep(Duration::from_millis(1));
                    }
                    wait_remote(&app);
                    app.process_authorization_with_projection(|o| config(o, files.root()));
                    assert!(
                        app.projection_sender_inclusion(&first),
                        "cancel keeps confirmed earlier target"
                    );
                }
            }
            if mode != 2 {
                assert!(matches!(app.projection_sender.phase, Phase::Checkpoint(_)));
                settle(&mut app);
                app.process_authorization_with_projection(|o| config(o, files.root()));
                assert!(app.projection_sender.group.is_none());
                assert!(matches!(app.projection_sender.phase, Phase::Idle));
                let entries = app
                    .workspace_checkpoint
                    .as_ref()
                    .unwrap()
                    .sender_index()
                    .unwrap()
                    .entries();
                assert_eq!(entries.len(), 2);
                for entry in entries {
                    assert!(app.projection_sender_inclusion(entry));
                }
                assert!(!ui::sender::groups::enabled(
                    &app.authorization.borrow(),
                    &app.documents[0].state.unit_id,
                    0
                ));
                assert!(app.authorization.borrow().sender.group_results[0].contains("2 关联已保存"));
                app.shutdown();
                drop(app);
                drain_messages();
                let doc = DocumentWindow::create(
                    GetModuleHandleW(None).unwrap().into(),
                    State::new(None, false).unwrap(),
                    false,
                )
                .unwrap();
                let mut reopened = Application::new(GetModuleHandleW(None).unwrap().into(), doc);
                reopened
                    .install_workspace_checkpoint(Some(Profile::open(files.root()).unwrap()), true)
                    .unwrap();
                assert_eq!(
                    reopened
                        .workspace_checkpoint
                        .as_ref()
                        .unwrap()
                        .sender_index()
                        .unwrap()
                        .entries()
                        .len(),
                    2
                );
                reopened.shutdown();
            } else {
                app.shutdown();
            }
        }
    }
    let received = server.join().unwrap();
    assert_eq!(
        received.len(),
        if mode == 2 || mode == 3 {
            1
        } else if mode == 4 {
            0
        } else {
            2
        }
    );
    assert_eq!(
        std::fs::read(files.root().join("device-identity.json")).unwrap(),
        before
    );
}
#[test]
fn group_menu_sends_each_target_after_canonical_and_reopens_both_bindings() {
    run(0);
}
#[test]
fn group_network_failure_pauses_and_explicit_retry_keeps_original_signed_request() {
    run(1);
}
#[test]
fn group_cancel_preserves_confirmed_target_cancels_pending_and_never_sends_queued_target() {
    run(2);
}
#[test]
fn group_late_result_after_content_change_is_cleaned_without_advancing_or_publishing() {
    run(3);
}
#[test]
fn group_over_capacity_fails_before_first_intent_or_network_send() {
    run(4);
}

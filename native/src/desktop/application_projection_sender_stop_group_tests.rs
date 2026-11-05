//! 组菜单停止的完整隔离链：逐项持久受理、真实canonical、TLS失败隔离及恢复；不发送系统输入。
use super::*;
#[path = "application_projection_sender_stop_group_guard_tests.rs"]
mod guard;
#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Complete,
    NetworkFailure,
    Cancel,
    BeginFailure,
    MissingIntent,
    LateSource,
}
fn journal(root: &std::path::Path) -> Value {
    serde_json::from_slice(&std::fs::read(root.join("projection-sender.json")).unwrap()).unwrap()
}
unsafe fn tick(app: &mut Application, root: &std::path::Path) {
    app.process_authorization_with_projection(|o| config(o, root));
}
unsafe fn until(
    app: &mut Application,
    root: &std::path::Path,
    check: impl Fn(&Application) -> bool,
) {
    let end = Instant::now() + Duration::from_secs(10);
    while !check(app) {
        settle(app);
        tick(app, root);
        assert!(
            Instant::now() < end,
            "group stop deadline: {:?}",
            app.authorization.borrow().sender.group_results
        );
        std::thread::sleep(Duration::from_millis(1));
    }
}
unsafe fn send_both(app: &mut Application, files: &Files, origin: &str) {
    start(app, files, origin, 2);
    for _ in 0..2 {
        wait_remote(app);
        tick(app, files.root());
        settle(app);
        tick(app, files.root());
    }
    assert!(app.projection_sender.group.is_none());
    assert_eq!(app.authorization.borrow().sender.bindings.len(), 2);
}
unsafe fn stop_menu(app: &Application) {
    let command = ui::sender::groups::STOP_BASE;
    assert!(crate::desktop::application::handles(command));
    assert!(crate::desktop::group_lock::allows(command));
    let menu = CreatePopupMenu().unwrap();
    ui::append_menu(menu, &app.documents[0].state).unwrap();
    assert_eq!(
        GetMenuState(menu, command as u32, MF_BYCOMMAND) & MF_GRAYED.0,
        0
    );
    DestroyMenu(menu).unwrap();
    ui::execute(app.documents[0].window.0, &app.documents[0].state, command).unwrap();
}
unsafe fn reopen(app: &mut Application, root: &std::path::Path) {
    app.shutdown();
    let instance = GetModuleHandleW(None).unwrap().into();
    let doc = DocumentWindow::create(instance, State::new(None, false).unwrap(), false).unwrap();
    let old = std::mem::replace(app, Application::new(instance, doc));
    drop(old);
    drain_messages();
    app.install_workspace_checkpoint(Some(Profile::open(root).unwrap()), true)
        .unwrap();
    app.projection_sender
        .start(root.to_owned(), app.execution_wake.callback());
    app.projection_sender.profile.collect(true);
}
fn run(mode: Mode) {
    let _serial = serial();
    let _com = Apartment::new().unwrap();
    let (files, identity, _) = Files::new();
    let identity_before = std::fs::read(files.root().join("device-identity.json")).unwrap();
    let (listener, client, tls, pem) = fixture_with_pem();
    let origin = client.origin().to_owned();
    std::fs::write(files.root().join("ca.pem"), pem).unwrap();
    let calls = match mode {
        Mode::Complete | Mode::NetworkFailure => 8,
        Mode::MissingIntent => 7,
        _ => 6,
    };
    let server = peer::start_stopping(
        listener,
        tls,
        files.root().to_owned(),
        calls,
        mode == Mode::NetworkFailure,
    );
    unsafe {
        let (_class, mut app) = application(files.root());
        authorize(&mut app, &files, identity, client);
        send_both(&mut app, &files, &origin);
        let source = app.restored_session.as_ref().unwrap().original.clone();
        let original = std::fs::read(files.root().join("projection-sender.json")).unwrap();
        if mode == Mode::MissingIntent {
            let request = app.projection_sender.profile.entries().unwrap()[0].clone();
            app.projection_sender.profile.retired(&request);
        }
        stop_menu(&app);
        if mode == Mode::BeginFailure {
            let mut changed = original.clone();
            changed.push(b' ');
            std::fs::write(files.root().join("projection-sender.json"), &changed).unwrap();
        }
        tick(&mut app, files.root());
        assert!(app.projection_sender.stop_group.is_some());
        if mode == Mode::Cancel || mode == Mode::LateSource {
            // 等Begin终态但不收割；取消/来源换代必须先于下一项受理。
            let end = Instant::now() + Duration::from_secs(5);
            while !app.projection_sender.stop.finished() {
                assert!(Instant::now() < end);
                std::thread::sleep(Duration::from_millis(1));
            }
            if mode == Mode::LateSource {
                app.restored_session
                    .as_mut()
                    .unwrap()
                    .edit_original(source.to_vec().into());
            } else {
                ui::execute(
                    app.documents[0].window.0,
                    &app.documents[0].state,
                    ui::sender::CANCEL,
                )
                .unwrap();
            }
            // 撤会话仅阻止网络，不撤已持久停止；不留下测试自动更新真实外部资源。
            app.authorization_owner.invalidate(&origin).unwrap();
            tick(&mut app, files.root());
            assert!(app.projection_sender.stop_group.is_none());
            assert!(!app.authorization.borrow().sender.group_stopping);
            let entries = journal(files.root());
            assert_eq!(entries["entries"][0]["stop"], "requested");
            assert_eq!(entries["entries"][1]["stop"], Value::Null);
            reopen(&mut app, files.root());
            until(&mut app, files.root(), |app| {
                app.workspace_checkpoint
                    .as_ref()
                    .unwrap()
                    .sender_index()
                    .unwrap()
                    .entries()
                    .iter()
                    .any(|b| b.stopped())
            });
            assert!(!app.remote_sessions.projection_pending());
            assert_eq!(
                app.workspace_checkpoint
                    .as_ref()
                    .unwrap()
                    .sender_index()
                    .unwrap()
                    .entries()
                    .len(),
                2
            );
            assert!(app
                .workspace_checkpoint
                .as_ref()
                .unwrap()
                .sender_index()
                .unwrap()
                .entries()
                .iter()
                .any(|b| b.target() == "receiver-1" && !b.stopped()));
        } else if mode == Mode::BeginFailure {
            until(&mut app, files.root(), |app| {
                app.projection_sender.stop_group.is_none()
            });
            assert!(app
                .restored_session
                .as_ref()
                .unwrap()
                .original
                .same_storage(&source));
            assert!(app.authorization.borrow().sender.group_results[0].contains("2 失败"));
            assert!(!app.remote_sessions.projection_pending());
            assert!(journal(files.root())["entries"]
                .as_array()
                .unwrap()
                .iter()
                .all(|entry| entry["stop"].is_null()));
            std::fs::write(files.root().join("projection-sender.json"), original).unwrap();
        } else {
            until(&mut app, files.root(), |app| {
                let results = &app.authorization.borrow().sender.group_results;
                if mode == Mode::Complete {
                    app.projection_sender.stop_group.is_none() && results[0].contains("2 已确认")
                } else {
                    results[0].contains("1 已确认")
                }
            });
            let remaining = journal(files.root());
            let index = app
                .workspace_checkpoint
                .as_ref()
                .unwrap()
                .sender_index()
                .unwrap();
            if mode == Mode::Complete {
                assert!(!app.authorization.borrow().sender.group_stopping);
                assert_eq!(remaining["entries"], json!([]));
                assert!(index.entries().is_empty());
                reopen(&mut app, files.root());
                assert!(app
                    .workspace_checkpoint
                    .as_ref()
                    .unwrap()
                    .sender_index()
                    .unwrap()
                    .entries()
                    .is_empty());
                assert!(app.projection_sender.profile.entries().unwrap().is_empty());
            } else {
                assert_eq!(index.entries().len(), 1);
                assert_eq!(index.entries()[0].target(), "receiver-0");
                if mode == Mode::NetworkFailure {
                    assert!(index.entries()[0].stopped());
                    assert_eq!(remaining["entries"][0]["stop"], "requested");
                    assert!(app.projection_sender.stop_group.is_some());
                    assert!(app.authorization.borrow().sender.group_results[0].contains("1 待确认"));
                } else {
                    assert!(!index.entries()[0].stopped());
                    assert!(app.authorization.borrow().sender.group_results[0].contains("1 失败"));
                    assert!(app.projection_sender.stop_group.is_none());
                }
                reopen(&mut app, files.root());
                assert_eq!(
                    app.workspace_checkpoint
                        .as_ref()
                        .unwrap()
                        .sender_index()
                        .unwrap()
                        .entries()
                        .len(),
                    1
                );
                assert!(!app.remote_sessions.projection_pending());
            }
        }
        app.shutdown();
    }
    assert_eq!(server.join().unwrap().len(), 2);
    assert_eq!(
        std::fs::read(files.root().join("device-identity.json")).unwrap(),
        identity_before
    );
}
#[test]
fn group_stop_confirms_each_canonical_removal_and_ack_then_reopens_empty() {
    run(Mode::Complete);
}
#[test]
fn group_stop_network_failure_does_not_starve_other_member_or_report_all_success() {
    run(Mode::NetworkFailure);
}
#[test]
fn group_stop_cancel_preserves_accepted_stop_but_never_marks_queued_member() {
    run(Mode::Cancel);
}
#[test]
fn group_stop_late_begin_after_source_replacement_cannot_advance_remaining_members() {
    run(Mode::LateSource);
}
#[test]
fn group_stop_failed_persistence_keeps_original_bindings_and_reports_each_failure() {
    run(Mode::BeginFailure);
}
#[test]
fn group_stop_missing_original_intent_does_not_block_valid_member() {
    run(Mode::MissingIntent);
}

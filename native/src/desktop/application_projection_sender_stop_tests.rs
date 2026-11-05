//! 真实停止菜单→持久减权→canonical pending→TLS unlink→回执→canonical 移除/ack；重启不借新授权。
use super::*;
use serde_json::{json, Value};
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
        tick(app, root);
        assert!(
            Instant::now() < end,
            "stable sender stop deadline: {}",
            app.authorization.borrow().sender.status
        );
        std::thread::sleep(Duration::from_millis(1));
    }
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
pub(super) unsafe fn run(
    app: &mut Application,
    root: &std::path::Path,
    binding: &Binding,
    mode: u8,
) {
    let before = std::fs::read(root.join("projection-sender.json")).unwrap();
    let source = app
        .workspace_checkpoint
        .as_ref()
        .unwrap()
        .sender_source()
        .unwrap()
        .clone();
    app.start_projection_sender_stop(ui::sender::StopSelection {
        binding: binding.clone(),
        source: source.to_vec().into(),
    });
    assert_eq!(
        std::fs::read(root.join("projection-sender.json")).unwrap(),
        before,
        "equal bytes cannot replace exact canonical allocation"
    );
    app.authorization.borrow_mut().targets.directory = None;
    tick(app, root);
    if mode == 8 {
        let mut changed = before.clone();
        changed.push(b' ');
        std::fs::write(root.join("projection-sender.json"), &changed).unwrap();
        ui::execute(
            app.documents[0].window.0,
            &app.documents[0].state,
            ui::sender::STOP_BASE,
        )
        .unwrap();
        tick(app, root);
        until(app, root, |app| {
            app.authorization
                .borrow()
                .sender
                .status
                .contains("持久减权失败")
        });
        assert!(!app.remote_sessions.projection_pending());
        assert!(
            app.projection_sender
                .deadline(now().unwrap())
                .is_none_or(|due| due > Instant::now()),
            "failed Begin cannot leave an expired wake starving real messages"
        );
        assert_eq!(
            std::fs::read(root.join("projection-sender.json")).unwrap(),
            changed
        );
        std::fs::write(root.join("projection-sender.json"), &before).unwrap();
        tick(app, root);
    }
    ui::execute(
        app.documents[0].window.0,
        &app.documents[0].state,
        ui::sender::STOP_BASE,
    )
    .unwrap();
    tick(app, root);
    until(app, root, |app| {
        let record = &app.restored_session.as_ref().unwrap().original;
        crate::projection_delivery::sender::binding::checkpoint::Index::capture(record)
            .is_some_and(|index| index.entries().iter().any(|binding| binding.stopped()))
    });
    assert!(
        !app.remote_sessions.projection_pending(),
        "memory pending is not canonical proof"
    );
    assert!(
        app.workspace_checkpoint
            .as_ref()
            .unwrap()
            .sender_source()
            .unwrap()
            .same_storage(&source)
            && app
                .workspace_checkpoint
                .as_ref()
                .unwrap()
                .sender_index()
                .unwrap()
                .includes(binding),
        "canonical still contains active binding before save"
    );
    assert_eq!(journal(root)["entries"][0]["stop"], "requested");
    settle(app);
    until(app, root, |app| app.remote_sessions.projection_pending());
    wait_remote(app);
    assert_eq!(journal(root)["entries"][0]["stop"], "unlinked");
    let original_envelope = journal(root)["entries"][0]["envelope"].clone();
    if mode == 7 {
        // 最危险窗口：HTTP+原日志事实已发布，canonical 仍 pending；新实例没有 token/目录。
        reopen(app, root);
        assert!(app
            .authorization_owner
            .lease(binding.envelope().origin(), now().unwrap())
            .is_err());
        assert_eq!(journal(root)["entries"][0]["envelope"], original_envelope);
        assert!(app
            .workspace_checkpoint
            .as_ref()
            .unwrap()
            .sender_index()
            .unwrap()
            .entries()[0]
            .stopped());
    }
    until(app, root, |app| {
        crate::projection_delivery::sender::binding::checkpoint::Index::capture(
            &app.restored_session.as_ref().unwrap().original,
        )
        .is_some_and(|index| index.entries().is_empty())
    });
    assert_eq!(
        journal(root)["entries"].as_array().unwrap().len(),
        1,
        "memory removal cannot acknowledge durable unlink receipt"
    );
    assert_ne!(app.authorization.borrow().sender.status, "发送关联已停止");
    settle(app);
    if mode == 9 {
        // 最终 canonical 已移除，但 ack 尚未调度；恢复仍只消费原持久成功事实。
        assert_eq!(journal(root)["entries"][0]["stop"], "unlinked");
        reopen(app, root);
        assert!(app
            .workspace_checkpoint
            .as_ref()
            .unwrap()
            .sender_index()
            .unwrap()
            .entries()
            .is_empty());
    }
    until(app, root, |app| {
        app.projection_sender.profile.entries().unwrap().is_empty()
    });
    assert_eq!(journal(root)["entries"], json!([]));
    assert_eq!(app.authorization.borrow().sender.status, "发送关联已停止");
    let saved: Value =
        serde_json::from_slice(&std::fs::read(root.join("workspace.json")).unwrap()).unwrap();
    assert_eq!(saved["stickers"][0]["projectionSenders"], json!([]));
    assert!(!app.remote_sessions.projection_pending());
    let source = app
        .workspace_checkpoint
        .as_ref()
        .unwrap()
        .sender_source()
        .unwrap();
    // 完整原意图（含显式离线路由）来自本测试停止前读到的写前记录，不按复合目标猜路由。
    let before: Value = serde_json::from_slice(&before).unwrap();
    let selection = serde_json::from_value::<crate::projection_delivery::sender::Selection>(
        before["entries"]
            .as_array()
            .unwrap()
            .iter()
            .find(|entry| entry["envelope"] == serde_json::to_value(binding.envelope()).unwrap())
            .unwrap()["selection"]
            .clone(),
    )
    .unwrap();
    assert!(
        Binding::admit_send(source, &selection).is_ok(),
        "confirmed stop allows a fresh send"
    );
    app.shutdown();
}
#[test]
fn native_sender_stable_stop_commits_pending_then_unlinks_and_acknowledges_exact_receipt() {
    tls::run_source(6, source::Case::Current);
}
#[test]
fn native_sender_stable_stop_restarts_after_unlink_receipt_without_network_or_new_identity() {
    tls::run_source(7, source::Case::Current);
}
#[test]
fn native_sender_stable_stop_failed_begin_preserves_original_and_can_explicitly_retry() {
    tls::run_source(8, source::Case::Current);
}
#[test]
fn native_sender_stable_stop_restarts_after_canonical_removal_before_ack() {
    tls::run_source(9, source::Case::Current);
}

//! 实际菜单/原Job/隔离TLS/Checkpoint源端闭环；不注册hook、不激活自有窗口或投送真实设备。
use super::*;
use crate::{
    editor::Tool,
    projection_edit as edit,
    stroke_style::{Color, StrokeStyle},
    viewport::Point,
};
use serde_json::{json, Value};
#[path = "application_projection_edit_automation_tests.rs"]
mod automation;
#[path = "application_projection_edit_source_guard_tests.rs"]
mod guard;
#[path = "application_projection_edit_source_peer.rs"]
mod peer;
unsafe fn mark(app: &Application) {
    let mut canvas = app.documents[0].state.canvas.borrow_mut();
    canvas
        .editor
        .set_style(StrokeStyle::new(2.0, Color(255, 0, 0)).unwrap());
    canvas
        .editor
        .begin(Tool::Rectangle, Point { x: 4.0, y: 4.0 })
        .unwrap();
    canvas.editor.extend(Point { x: 12.0, y: 12.0 }).unwrap();
    assert!(canvas.editor.commit());
}
unsafe fn send(app: &mut Application, files: &Files, index: usize) {
    ui::execute(
        app.documents[0].window.0,
        &app.documents[0].state,
        ui::sender::BASE + index,
    )
    .unwrap();
    app.process_authorization_with_projection(|o| config(o, files.root()));
    wait_remote(app);
    app.process_authorization_with_projection(|o| config(o, files.root()));
    assert!(
        matches!(app.projection_sender.phase, Phase::Checkpoint(_)),
        "{}",
        app.authorization.borrow().sender.status
    );
    settle(app);
    app.poll_projection_sender(&mut |o| config(o, files.root()));
    assert!(!app.projection_sender.busy());
}
unsafe fn begin(app: &mut Application, files: &Files) {
    ui::execute(
        app.documents[0].window.0,
        &app.documents[0].state,
        ui::editing::INITIALIZE,
    )
    .unwrap();
    app.projection_edit_requests(&mut |o| config(o, files.root()));
    assert!(
        app.projection_edit.busy(),
        "{}",
        app.authorization.borrow().editing.status
    );
}
unsafe fn completed(app: &Application) {
    let handle = app.projection_edit.wait_handle().unwrap();
    assert_eq!(
        WaitForSingleObject(HANDLE(handle.as_raw_handle()), 5000),
        WAIT_OBJECT_0
    );
}
unsafe fn poll(app: &mut Application, files: &Files) {
    app.poll_projection_edit(&mut |o| config(o, files.root()));
}
unsafe fn finish(app: &mut Application, files: &Files) {
    let end = Instant::now() + Duration::from_secs(10);
    while app.projection_edit.busy() {
        poll(app, files);
        settle(app);
        assert!(
            Instant::now() < end,
            "{}",
            app.authorization.borrow().editing.status
        );
        std::thread::yield_now();
    }
}
fn journal(files: &Files) -> Value {
    serde_json::from_slice(&std::fs::read(files.root().join("projection-edit.json")).unwrap())
        .unwrap()
}
fn snapshot(app: &Application) -> edit::workspace::Snapshot {
    app.documents[0]
        .state
        .projection_edit
        .borrow()
        .as_ref()
        .unwrap()
        .capture(&app.documents[0].state.canvas.borrow().editor)
}
unsafe fn initialized(
    app: &mut Application,
    files: &Files,
    identity: crate::device_identity_store::Store,
    client: crate::device_session_client::Client,
    count: usize,
) {
    let origin = client.origin().to_owned();
    groups::authorize(app, files, identity, client);
    app.authorization.borrow_mut().targets.directory = Some(groups::directory(&origin, count));
    mark(app);
    for index in 0..count {
        send(app, files, index);
    }
    begin(app, files);
    completed(app);
    poll(app, files);
    assert!(journal(files)["entries"][0]["pending"].is_object());
    // 真实commit前的额外轮询不能丢失owner，也不能把候选索引当durable凭证退休pending。
    assert!(app.projection_sender_scope().is_none());
    for _ in 0..2 {
        poll(app, files);
        assert!(
            app.projection_edit.busy(),
            "{}",
            app.authorization.borrow().editing.status
        );
        assert!(
            app.projection_edit.wait_handle().is_none(),
            "still awaiting checkpoint, not ack"
        );
        assert!(journal(files)["entries"][0]["pending"].is_object());
    }
    finish(app, files);
    assert!(
        journal(files)["entries"][0]["pending"].is_null(),
        "{}",
        app.authorization.borrow().editing.status
    );
}
#[test]
fn source_menu_freezes_basis_initializes_existing_targets_and_later_target_attaches_same_session() {
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
        initialized(&mut app, &files, identity, client, 2);
        let state = snapshot(&app);
        assert_eq!(state.role(), edit::workspace::Role::Source);
        assert_eq!(state.document().mode(), edit::Mode::OneWay);
        let frozen = journal(&files)["entries"][0]["source"].clone();
        assert_eq!(frozen["initial"].as_object().unwrap().len(), 1);
        let original = std::fs::read(files.root().join("projection-sender.json")).unwrap();
        assert!(app.documents[0]
            .state
            .canvas
            .borrow_mut()
            .editor
            .translate(0, Point { x: 2.0, y: 0.0 })
            .unwrap());
        app.authorization.borrow_mut().targets.directory = Some(groups::directory(&origin, 3));
        send(&mut app, &files, 2);
        let unit = app.documents[0].state.unit_id.clone();
        let targets = app.projection_source_targets(&unit, &origin);
        let latest = targets
            .iter()
            .find(|t| t.binding.target() == "receiver-2")
            .unwrap()
            .binding
            .envelope()
            .projection_id()
            .to_owned();
        assert!(
            app.start_projection_source(unit, Some(vec![latest]), false, &mut |o| config(
                o,
                files.root()
            ))
        );
        finish(&mut app, &files);
        assert_eq!(journal(&files)["entries"][0]["source"], frozen);
        assert_eq!(
            snapshot(&app).document().session_id(),
            state.document().session_id()
        );
        assert!(
            !app.projection_edit.busy(),
            "attach completion automatically submits local edits"
        );
        assert_eq!(
            snapshot(&app).document().revision(),
            2,
            "source may write one_way"
        );
        let trace = peer.trace.lock().unwrap();
        assert_eq!(
            trace
                .iter()
                .filter(|e| e["body"]["operation"] == "initialize")
                .count(),
            1
        );
        assert_eq!(
            trace
                .iter()
                .filter(|e| e["body"]["operation"] == "attach")
                .count(),
            2
        );
        assert_eq!(
            trace
                .iter()
                .filter(|e| e["path"] == "/v1/projections/update")
                .count(),
            2,
            "only old flattened targets need a PNG CAS"
        );
        assert_eq!(
            trace
                .iter()
                .find(|e| e["body"]["operation"] == "apply")
                .unwrap()["body"]["changes"][0]["value"]["x"],
            6.0
        );
        drop(trace);
        assert_ne!(
            std::fs::read(files.root().join("projection-sender.json")).unwrap(),
            original,
            "only third target creates new signed intent"
        );
        let saved = snapshot(&app);
        app.shutdown();
        drop(app);
        drain_messages();
        let instance = GetModuleHandleW(None).unwrap().into();
        let doc =
            DocumentWindow::create(instance, State::new(None, false).unwrap(), false).unwrap();
        let mut reopened = Application::new(instance, doc);
        reopened
            .install_workspace_checkpoint(Some(Profile::open(files.root()).unwrap()), true)
            .unwrap();
        assert_eq!(snapshot(&reopened).document(), saved.document());
        assert_eq!(snapshot(&reopened).role(), edit::workspace::Role::Source);
        assert!(!reopened.projection_edit.busy());
        assert!(!reopened.remote_sessions.projection_pending());
        reopened.shutdown();
    }
}

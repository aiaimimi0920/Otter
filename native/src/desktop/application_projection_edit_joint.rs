//! 双真实Loom的Application联合夹具；只复用原菜单、调度、Checkpoint和有界隔离身份。
use super::*;
use crate::device_authorization_config::Action;
use crate::{projection_edit as edit, viewport::Point};
#[path = "application_projection_edit_joint_focus.rs"]
mod focus;
#[path = "application_projection_edit_joint_receiver.rs"]
mod receiver;
#[path = "application_projection_edit_joint_source.rs"]
mod source;

unsafe fn tick(app: &mut Application, f: &fixture::Fixture) {
    app.defer_projection_poll_for_test(&f.origin);
    process(app, f);
    app.pending_workspace_save_with(|_, _, message, warning| assert!(!warning, "{message}"));
    drain_messages(); // 与真实UI线程一致投递自有窗口消息和被动WinEvent；不操作用户窗口。
}
unsafe fn until(app: &mut Application, f: &fixture::Fixture, done: impl Fn(&Application) -> bool) {
    let end = Instant::now() + Duration::from_secs(10);
    loop {
        tick(app, f);
        if done(app) {
            return;
        }
        assert!(
            Instant::now() < end,
            "application edit joint deadline: edit={} sender={} receiver={}",
            app.authorization.borrow().editing.status,
            app.authorization.borrow().sender.status,
            app.authorization.borrow().projection.status
        );
        std::thread::sleep(Duration::from_millis(1));
    }
}
fn quiet(app: &Application) -> bool {
    !app.projection_edit.busy()
        && !app.projection_sender.busy()
        && !app.projection_receive.busy()
        && !app.remote_sessions.projection_pending()
        && app.save_job.is_none()
}
fn state(app: &Application) -> &State {
    &app.documents
        .iter()
        .find(|d| d.state.projection_edit.borrow().is_some())
        .unwrap()
        .state
}
fn snapshot(app: &Application) -> edit::workspace::Snapshot {
    let state = state(app);
    state
        .projection_edit
        .borrow()
        .as_ref()
        .unwrap()
        .capture(&state.canvas.borrow().editor)
}
fn has_revision(app: &Application, revision: u64) -> bool {
    app.documents.iter().any(|d| {
        d.state.projection_edit.borrow().as_ref().is_some_and(|v| {
            v.capture(&d.state.canvas.borrow().editor)
                .document()
                .revision()
                == revision
        })
    })
}
unsafe fn command(app: &mut Application, f: &fixture::Fixture, command: usize) {
    let doc = app
        .documents
        .iter()
        .find(|d| d.state.projection_edit.borrow().is_some())
        .unwrap_or(&app.documents[0]);
    ui::execute(doc.window.0, &doc.state, command).unwrap();
    tick(app, f);
}
unsafe fn renew(app: &mut Application, f: &fixture::Fixture) {
    app.authorization.borrow_mut().origin = Some(f.origin.clone());
    app.authorization_owner
        .start(Task::Configured {
            config: config(&f.origin, &f.profile).unwrap(),
            action: Action::Renew,
        })
        .unwrap();
    let handle = app.authorization_owner.wait_handles().next().unwrap();
    assert_eq!(
        WaitForSingleObject(HANDLE(handle.as_raw_handle()), 5000),
        WAIT_OBJECT_0
    );
    tick(app, f);
}
unsafe fn reopen(app: &mut Application, f: &fixture::Fixture, receiver: bool) {
    app.shutdown();
    let instance = GetModuleHandleW(None).unwrap().into();
    let doc = DocumentWindow::create(instance, State::new(None, false).unwrap(), false).unwrap();
    let old = std::mem::replace(app, Application::new(instance, doc));
    drop(old);
    drain_messages();
    app.install_workspace_checkpoint(Some(Profile::open(&f.profile).unwrap()), true)
        .unwrap();
    app.projection_sender
        .start(f.profile.clone(), app.execution_wake.callback());
    app.projection_sender.profile.collect(true);
    if receiver {
        app.projection_profile
            .start(f.profile.clone(), app.execution_wake.callback());
        until(app, f, |app| {
            matches!(
                app.projection_profile,
                crate::desktop::application::projection::profile::Owner::Ready { .. }
            )
        });
    } else {
        app.authorization.borrow_mut().consent.install_paused(true);
    }
    renew(app, f);
}
unsafe fn shift(app: &Application, dx: f32) {
    assert!(state(app)
        .canvas
        .borrow_mut()
        .editor
        .translate(0, Point { x: dx, y: 0.0 })
        .unwrap());
}
fn verify(app: &Application, revision: u64, mode: edit::Mode, x: f64) -> Value {
    let snapshot = snapshot(app);
    assert_eq!(snapshot.document().revision(), revision);
    assert_eq!(snapshot.document().mode(), mode);
    assert!(snapshot.suspended().is_none());
    let objects = snapshot.editor_view().unwrap().basis_objects().clone();
    assert_eq!(objects.len(), 1);
    let value = objects.values().next().unwrap();
    assert_eq!(value["x"].as_f64(), Some(x));
    assert_eq!(value["y"].as_f64(), Some(4.0));
    assert_eq!(value["w"].as_f64(), Some(8.0));
    assert_eq!(value["h"].as_f64(), Some(8.0));
    serde_json::to_value(snapshot.document()).unwrap()
}
fn acknowledged(f: &fixture::Fixture, revision: u64) {
    let value: Value =
        serde_json::from_slice(&std::fs::read(f.profile.join("projection-edit.json")).unwrap())
            .unwrap();
    assert_eq!(value["entries"].as_array().unwrap().len(), 1);
    assert!(value["entries"][0]["pending"].is_null());
    assert_eq!(value["entries"][0]["document"]["revision"], revision);
}
unsafe fn frame(app: &Application, f: &fixture::Fixture, name: &str) {
    use std::io::Write;
    let canvas = state(app).canvas.borrow();
    let image = canvas.minified_source.as_ref().unwrap().image();
    let (png, _) = crate::desktop::export::png_with_opacity(image, &canvas.editor, 1.0).unwrap();
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(f.root.join(name))
        .unwrap();
    file.write_all(&png).unwrap();
    file.sync_all().unwrap();
}

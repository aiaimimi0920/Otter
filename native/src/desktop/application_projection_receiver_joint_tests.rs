//! 两真实Loom的原生接收消费者：菜单/后台接纳/Checkpoint/真实帧后回执/持续更新/真正重开。
use super::*;
use crate::desktop::application::projection;
use crate::device_authorization_config::Action;
use std::sync::atomic::AtomicBool;

unsafe fn tick(app: &mut Application, f: &fixture::Fixture) {
    app.defer_projection_poll_for_test(&f.origin);
    process(app, f);
    app.pending_workspace_save_with(|_, _, message, warning| assert!(!warning, "{message}"));
}
unsafe fn until(app: &mut Application, f: &fixture::Fixture, done: impl Fn(&Application) -> bool) {
    let end = Instant::now() + Duration::from_secs(10);
    loop {
        tick(app, f);
        if done(app) {
            break;
        }
        assert!(
            Instant::now() < end,
            "real Loom receiver deadline: {}",
            app.authorization.borrow().projection.status
        );
        std::thread::sleep(Duration::from_millis(1));
    }
}
pub(super) fn received(app: &Application) -> Option<Value> {
    let value: Value = serde_json::from_slice(&app.restored_session.as_ref()?.original).unwrap();
    value["stickers"]
        .as_array()?
        .iter()
        .find(|v| v.get("qrProjection").is_some_and(Value::is_object))
        .cloned()
}
fn verify(app: &Application, f: &fixture::Fixture, revision: u64) -> Value {
    let row = received(app).unwrap();
    let binding = &row["qrProjection"];
    assert_eq!(binding["revision"], revision);
    assert_eq!(binding["envelope"]["serverOrigin"], f.source);
    assert_ne!(f.source, f.origin);
    assert_eq!(binding["offlineTransport"]["origin"], f.origin);
    assert_eq!(
        binding["otterNativeReceive"]["peerId"],
        f.peer.as_deref().unwrap()
    );
    let image = crate::desktop::session_image::inline(
        row["src"].as_str().unwrap(),
        &AtomicBool::new(false),
    )
    .unwrap();
    let (size, pixel) = if revision == 1 {
        ((32, 32), [255, 0, 0, 255])
    } else {
        ((16, 8), [0, 255, 0, 255])
    };
    assert_eq!((image.width, image.height), size);
    assert!(image.pixels.chunks_exact(4).all(|p| p == pixel));
    row
}
unsafe fn reopen(app: &mut Application, f: &fixture::Fixture) {
    app.shutdown();
    let instance = GetModuleHandleW(None).unwrap().into();
    let doc = DocumentWindow::create(instance, State::new(None, false).unwrap(), false).unwrap();
    let old = std::mem::replace(app, Application::new(instance, doc));
    drop(old);
    drain_messages();
    app.install_workspace_checkpoint(Some(Profile::open(&f.profile).unwrap()), true)
        .unwrap();
    app.projection_profile
        .start(f.profile.clone(), app.execution_wake.callback());
    app.authorization.borrow_mut().origin = Some(f.origin.clone());
    until(app, f, |app| {
        matches!(
            app.projection_profile,
            projection::profile::Owner::Ready { .. }
        )
    });
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

#[test]
#[ignore = "launched only by isolated two real Loom HTTPS offline fixture"]
fn native_receiver_joint_two_loom_menu_display_update_reopen_and_terminal() {
    let _serial = serial();
    let _com = Apartment::new().unwrap();
    let (f, client, store) = fixture::Fixture::load();
    assert!(f.peer.is_some());
    unsafe {
        let (_class, mut app) = application(&f.profile);
        app.projection_profile
            .start(f.profile.clone(), app.execution_wake.callback());
        app.authorization.borrow_mut().origin = Some(f.origin.clone());
        until(&mut app, &f, |app| {
            matches!(
                app.projection_profile,
                projection::profile::Owner::Ready { .. }
            )
        });
        app.authorization_owner
            .start(Task::Renew { client, store })
            .unwrap();
        let handle = app.authorization_owner.wait_handles().next().unwrap();
        assert_eq!(
            WaitForSingleObject(HANDLE(handle.as_raw_handle()), 5000),
            WAIT_OBJECT_0
        );
        tick(&mut app, &f);
        // 使用真实策略菜单及后台原子提交，不只把内存Consent改成允许。
        ui::execute(
            app.documents[0].window.0,
            &app.documents[0].state,
            ui::policy::CONFIRM,
        )
        .unwrap();
        until(&mut app, &f, |app| {
            app.authorization.borrow().consent.stamp().is_some()
        });
        ui::execute(
            app.documents[0].window.0,
            &app.documents[0].state,
            ui::projection::REFRESH,
        )
        .unwrap();
        tick(&mut app, &f);
        wait_remote(&app);
        tick(&mut app, &f);
        assert_eq!(app.authorization.borrow().projection.entries.len(), 1);
        let entry = app.authorization.borrow().projection.entries[0].clone();
        assert_eq!(entry.origin(), f.origin);
        assert_eq!(entry.peer_id(), f.peer.as_deref());
        assert_eq!(entry.envelope().origin(), f.source);
        let foreground = GetForegroundWindow();
        ui::projection::accept::confirm_with(
            app.documents[0].window.0,
            &app.documents[0].state,
            ui::projection::ACCEPT_BASE,
            |_| true,
        )
        .unwrap();
        until(&mut app, &f, |app| {
            app.authorization.borrow().projection.status == "投送已呈现"
        });
        assert_eq!(
            GetForegroundWindow(),
            foreground,
            "receiver must not activate"
        );
        let row = verify(&app, &f, 1);
        let unit = row["id"].as_str().unwrap().to_owned();
        let doc = app
            .documents
            .iter()
            .find(|d| d.state.unit_id.as_str() == unit)
            .unwrap();
        assert!(IsWindowVisible(doc.window.0).as_bool());
        assert!(doc.state.canvas.borrow().confirmed_paints > 0);
        let disk: Value =
            serde_json::from_slice(&std::fs::read(f.profile.join("workspace.json")).unwrap())
                .unwrap();
        assert!(disk["stickers"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v["qrProjection"] == row["qrProjection"]));
        f.publish("displayed.json",&json!({"binding":row["qrProjection"],"saved":true,"realFrame":true,"foregroundUnchanged":true}));
        f.wait("updated");
        until(&mut app, &f, |app| {
            received(app).is_some_and(|v| v["qrProjection"]["revision"] == 2)
                && !app.projection_receive.busy()
        });
        verify(&app, &f, 2);
        app.start_checkpoint_with(&mut |_, _, message, warning| assert!(!warning, "{message}"));
        settle(&mut app);
        reopen(&mut app, &f);
        let updated = verify(&app, &f, 2);
        assert_eq!(updated["id"], unit);
        f.publish("updated.json",&json!({"binding":updated["qrProjection"],"reopened":true,"pixelsMatch":true,"width":16,"height":8}));
        f.wait("terminal");
        until(&mut app, &f, |app| {
            received(app).is_some_and(|v| v["qrProjection"]["stopped"] == true)
                && !app.projection_receive.busy()
        });
        let last = verify(&app, &f, 2);
        assert_eq!(
            last["qrProjection"]["stopReason"],
            if f.revoke {
                "projection_peer_revoked"
            } else {
                "projection_unlinked"
            }
        );
        app.start_checkpoint_with(&mut |_, _, message, warning| assert!(!warning, "{message}"));
        settle(&mut app);
        reopen(&mut app, &f);
        assert_eq!(verify(&app, &f, 2)["qrProjection"], last["qrProjection"]);
        app.shutdown();
        drop(app);
        drain_messages();
        f.publish(
            "terminal.json",
            &json!({"binding":last["qrProjection"],"reopened":true,"lastImagePreserved":true}),
        );
    }
}

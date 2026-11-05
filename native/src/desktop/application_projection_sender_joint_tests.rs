//! 显式真实Loom HTTPS联合消费者：原菜单/Job/PNG/版本/Checkpoint/停止，不冒充可见或跨机验收。
use super::*;
use serde_json::{json, Value};
#[path = "application_projection_edit_joint.rs"]
mod collaboration;
#[path = "application_projection_sender_joint_fixture.rs"]
mod fixture;
#[path = "application_projection_receiver_joint_tests.rs"]
mod receiver;

unsafe fn binding(app: &Application) -> Binding {
    app.workspace_checkpoint
        .as_ref()
        .unwrap()
        .sender_index()
        .unwrap()
        .entries()[0]
        .clone()
}
unsafe fn process(app: &mut Application, f: &fixture::Fixture) {
    app.process_authorization_with_projection(|origin| config(origin, &f.profile));
}
unsafe fn cycle(app: &mut Application, f: &fixture::Fixture) {
    app.projection_sender.updates.make_due();
    process(app, f);
    assert!(matches!(app.projection_sender.phase, Phase::Updating(_)));
    wait_remote(app);
    process(app, f);
    assert!(
        matches!(app.projection_sender.phase, Phase::Checkpoint(_)),
        "{}",
        app.authorization.borrow().sender.status
    );
    settle(app);
    process(app, f);
}
#[test]
#[ignore = "launched only by isolated real Loom HTTPS projection fixture"]
fn native_sender_joint_real_loom_updates_reads_stops_and_restores() {
    run(false);
}
#[test]
#[ignore = "launched only by isolated two real Loom HTTPS offline fixture"]
fn native_sender_joint_two_loom_offline_updates_and_stops() {
    run(true);
}
fn run(offline: bool) {
    let _serial = serial();
    let _com = Apartment::new().unwrap();
    let (f, client, store) = fixture::Fixture::load();
    assert_eq!(f.peer.is_some(), offline);
    unsafe {
        let (_class, mut app) = application(&f.profile);
        app.authorization.borrow_mut().origin = Some(f.origin.clone());
        app.authorization.borrow_mut().consent.install_paused(true);
        app.authorization_owner
            .start(Task::Renew { client, store })
            .unwrap();
        let handle = app.authorization_owner.wait_handles().next().unwrap();
        assert_eq!(
            WaitForSingleObject(HANDLE(handle.as_raw_handle()), 5000),
            WAIT_OBJECT_0
        );
        process(&mut app, &f);
        app.authorization.borrow_mut().request = Some(ui::Request::ProjectionTargets);
        process(&mut app, &f);
        wait_remote(&app);
        process(&mut app, &f);
        let target = app
            .authorization
            .borrow()
            .targets
            .directory
            .as_ref()
            .unwrap()
            .targets()
            .iter()
            .position(|target| target.id() == f.target)
            .unwrap();
        ui::execute(
            app.documents[0].window.0,
            &app.documents[0].state,
            ui::sender::BASE + target,
        )
        .unwrap();
        process(&mut app, &f);
        wait_remote(&app);
        process(&mut app, &f);
        assert!(
            matches!(app.projection_sender.phase, Phase::Checkpoint(_)),
            "{}",
            app.authorization.borrow().sender.status
        );
        settle(&mut app);
        process(&mut app, &f);
        let first = binding(&app);
        if offline {
            let value = serde_json::to_value(&first).unwrap();
            assert_eq!(value["link"]["offlineTransport"]["origin"], f.origin);
            assert_eq!(value["target"]["peerId"], f.peer.as_deref().unwrap());
            assert_eq!(first.envelope().origin(), f.source);
        }
        assert_eq!(first.revision(), 1);
        assert!(!first.linked());
        let journal = std::fs::read(f.profile.join("projection-sender.json")).unwrap();
        f.publish("created.json", &json!({"envelope":first.envelope(),"revision":first.revision(),"digest":first.digest()}));
        f.wait("accepted");
        // 远端接纳只同步原关联的回执，不创建第二邀请或伪造displayed。
        cycle(&mut app, &f);
        assert!(binding(&app).linked());
        assert_eq!(binding(&app).revision(), 1);
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
        // 保持真实服务原500ms速率限制；只控制测试步骤，不改生产调度/期限。
        std::thread::sleep(Duration::from_millis(550));
        cycle(&mut app, &f);
        let updated = binding(&app);
        assert_eq!(updated.revision(), 2);
        assert_eq!(updated.envelope(), first.envelope());
        assert_ne!(updated.digest(), first.digest());
        assert_eq!(
            std::fs::read(f.profile.join("projection-sender.json")).unwrap(),
            journal
        );
        let saved: Value =
            serde_json::from_slice(&std::fs::read(f.profile.join("workspace.json")).unwrap())
                .unwrap();
        assert_eq!(
            saved["stickers"][0]["projectionSenders"][0]["link"]["revision"],
            2
        );
        f.publish("updated.json", &json!({"envelope":updated.envelope(),"revision":updated.revision(),"digest":updated.digest(),"width":16,"height":8}));
        f.wait("read-verified");
        if f.revoke {
            // 撤销由真实Loom执行；保留已保存来源，不用失败unlink伪造已停止回执。
            f.wait("peer-revoked");
            let path = f.profile.join("workspace.json");
            let before = std::fs::read(&path).unwrap();
            app.shutdown();
            assert_eq!(std::fs::read(&path).unwrap(), before);
            let saved: Value = serde_json::from_slice(&before).unwrap();
            let row = &saved["stickers"][0];
            let persisted: Binding =
                serde_json::from_value(row["projectionSenders"][0].clone()).unwrap();
            assert!(persisted == updated);
            for (field, expected) in [("x", 4.0), ("y", 2.0), ("w", 16.0), ("h", 8.0)] {
                assert_eq!(
                    row["imageEditState"]["cropRect"][field].as_f64(),
                    Some(expected)
                );
            }
            let image = crate::desktop::session_image::inline(
                row["src"].as_str().unwrap(),
                &std::sync::atomic::AtomicBool::new(false),
            )
            .unwrap();
            assert_eq!((image.width, image.height), (32, 32));
            assert!(image.pixels.chunks_exact(4).all(|p| p == [0, 255, 0, 255]));
            f.publish(
                "revoked.json",
                &json!({"localContentPreserved":true,"revision":2}),
            );
            return;
        }
        // 复用原停止恢复窗口：真实unlink完成、canonical移除后重开，无新token也能消费原持久回执。
        stop::run(&mut app, &f.profile, &updated, 9);
        assert!(app
            .workspace_checkpoint
            .as_ref()
            .unwrap()
            .sender_index()
            .unwrap()
            .entries()
            .is_empty());
        app.shutdown();
        drop(app);
        drain_messages();
        f.publish(
            "stopped.json",
            &json!({"stopped":true,"restoredReceipt":true,"revision":2}),
        );
    }
}

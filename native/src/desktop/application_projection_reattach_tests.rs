//! 自有隐藏 Application 的 accepted 菜单/目标/保存/呈现链；NEW 策略不是既有关联撤权。
use super::*;
use crate::{projection_delivery::accept::tests as png, projection_receive_policy::Policy};
#[path = "application_projection_poll_reattach_tests.rs"]
mod automatic;
#[path = "application_projection_reattach_lifecycle_tests.rs"]
mod lifecycle;

unsafe fn fixture() -> Fixture {
    Fixture::with_delivery(true, |stream, origin| {
        let mut value = png::response(origin);
        value["receiverDeviceId"] = "device-old".into();
        reply(stream, 200, &serde_json::to_vec(&value).unwrap());
    })
}
fn entry(fixture: &mut Fixture) -> Arc<crate::projection_delivery::Invitation> {
    let entry = Arc::new(if fixture.offline {
        crate::projection_delivery::tests::offline::invitation(&fixture.origin, true)
    } else {
        png::accepted(&fixture.origin)
    });
    fixture.app.authorization.borrow_mut().projection.entries = vec![entry.clone()];
    entry
}
fn poll(fixture: &mut Fixture) {
    fixture
        .app
        .process_authorization_with_projection(|origin| config(origin, fixture.files.root()));
}
unsafe fn select(fixture: &mut Fixture) {
    entry(fixture);
    let doc = &fixture.app.documents[0];
    ui::accept::confirm_with(doc.window.0, &doc.state, ui::REATTACH_BASE, |_| true).unwrap();
    assert!(matches!(
        fixture.app.authorization.borrow().request,
        Some(Request::Projection(ui::Request::Reattach(_)))
    ));
    poll(fixture);
    assert!(fixture.app.remote_sessions.projection_pending());
    wait_remote(&fixture.app);
    fixture.finish_accept();
}
fn receipt(fixture: &mut Fixture) {
    let listener = fixture.listener.take().unwrap();
    let tls = fixture.tls.clone();
    let origin = fixture.origin.clone();
    let offline = fixture.offline;
    fixture.server = Some(std::thread::spawn(move || {
        let mut stream = StreamOwned::new(ServerConnection::new(tls).unwrap(), accept(&listener));
        let (head, body) = request(&mut stream);
        assert!(head.starts_with(if offline {
            "POST /v1/offline-projections/receipt "
        } else {
            "POST /v1/projections/receipt "
        }));
        assert_eq!(
            body,
            serde_json::json!({"projectionId":png::invitation(&origin).envelope().projection_id(),"status":"displayed"})
        );
        reply(&mut stream, 200, br#"{"recorded":true}"#);
        listener
    }));
}

#[test]
fn projection_reattach_native_reject_policy_still_restores_saves_and_presents_existing_receiver() {
    unsafe { run_routed(false) }
}
unsafe fn run_routed(offline: bool) {
    let _serial = crate::loom_bridge_client::tests::execution_job::SERIAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let _com = Apartment::new().unwrap();
    unsafe {
        let mut fixture = if offline {
            Fixture::offline(true)
        } else {
            fixture()
        };
        fixture
            .app
            .authorization
            .borrow_mut()
            .consent
            .install(Policy::Reject);
        let count = fixture.app.documents.len();
        select(&mut fixture);
        assert!(!fixture.unit_present());
        assert!(!fixture.app.projection_cleanup.pending_local());
        // 网络已结束而尚未领取；新的策略代次/保存中状态不能撤销旧 receiver。
        fixture.app.authorization.borrow_mut().consent.begin_save();
        until(&mut fixture.app, |app| {
            !app.projection_receive.preparing() && !app.remote_sessions.projection_pending()
        });
        assert!(fixture.unit_present());
        assert_eq!(fixture.app.documents.len(), count + 1);
        let doc = fixture
            .app
            .documents
            .iter()
            .find(|doc| doc.state.unit_id.as_str() == fixture.ticket().unit())
            .unwrap();
        assert!(!IsWindowVisible(doc.window.0).as_bool());
        assert!(fixture
            .app
            .projection_checkpoint_inclusion(fixture.ticket())
            .is_none());
        assert!(!fixture.ticket().cleanup_pending());
        // checkpoint 之前 listener 无 receipt；显式保存后才允许原呈现围栏。
        assert!(
            matches!(fixture.listener.as_ref().unwrap().accept(), Err(e) if e.kind() == std::io::ErrorKind::WouldBlock)
        );
        fixture.app.authorization.borrow_mut().consent.saving = false;
        fixture
            .app
            .authorization
            .borrow_mut()
            .consent
            .install(Policy::Confirm);
        receipt(&mut fixture);
        fixture
            .app
            .start_checkpoint_with(&mut |_, _, message, warning| assert!(!warning, "{message}"));
        checkpoint(&mut fixture.app);
        let end = Instant::now() + Duration::from_secs(10);
        while fixture.app.projection_receive.busy() {
            poll(&mut fixture);
            assert!(Instant::now() < end, "accepted reattach display deadline");
            std::thread::yield_now();
        }
        fixture.listener = Some(fixture.server.take().unwrap().join().unwrap());
        assert_eq!(
            fixture.app.authorization.borrow().projection.status,
            "投送已呈现"
        );
        assert!(fixture
            .app
            .projection_checkpoint_inclusion(fixture.ticket())
            .is_some());
        assert_eq!(fixture.store.pending().unwrap().len(), 1);
        assert!(!fixture.app.projection_cleanup.pending_local());
        fixture.app.reset_projection_inbox();
        entry(&mut fixture);
        let doc = &fixture.app.documents[0];
        ui::accept::confirm_with(doc.window.0, &doc.state, ui::REATTACH_BASE, |_| true).unwrap();
        fixture.app.process_authorization_with_projection(|_| {
            panic!("same UnitId cannot read/append twice")
        });
        assert_eq!(fixture.app.documents.len(), count + 1);
        assert!(!fixture.app.remote_sessions.projection_pending());
    }
}
#[test]
fn projection_offline_reattach_menu_reads_saves_and_presents_existing_receiver_without_accept() {
    unsafe { run_routed(true) }
}

#[test]
fn projection_reattach_native_menu_revalidates_arc_and_never_treats_new_as_existing() {
    let _com = Apartment::new().unwrap();
    unsafe {
        let (_class, mut app) = application();
        let origin = "https://loom.example";
        let accepted = Arc::new(png::accepted(origin));
        let mut view = app.authorization.borrow_mut();
        view.origin = Some(origin.into());
        view.consent.install(Policy::Reject);
        view.projection.entries = vec![accepted.clone()];
        drop(view);
        let doc = &app.documents[0];
        ui::accept::confirm_with(doc.window.0, &doc.state, ui::REATTACH_BASE, |_| false).unwrap();
        assert!(app.authorization.borrow().request.is_none());
        assert!(
            ui::accept::confirm_with(doc.window.0, &doc.state, ui::REATTACH_BASE, |_| {
                app.authorization.borrow_mut().projection.entries[0] =
                    Arc::new((*accepted).clone());
                true
            })
            .is_err()
        );
        app.authorization.borrow_mut().projection.entries = vec![Arc::new(png::invitation(origin))];
        assert!(
            ui::accept::confirm_with(doc.window.0, &doc.state, ui::REATTACH_BASE, |_| panic!(
                "NEW cannot prompt as accepted"
            ))
            .is_err()
        );
        // 直接排队也必须在配置/网络前再次拒绝 NEW。
        app.authorization.borrow_mut().request = Some(Request::Projection(ui::Request::Reattach(
            Arc::new(png::invitation(origin)),
        )));
        app.process_authorization_with_projection(|_| {
            panic!("invalid existing request reached config")
        });
        assert!(!app.remote_sessions.projection_pending());
        app.shutdown();
    }
}

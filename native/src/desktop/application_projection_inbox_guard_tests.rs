//! UI 排队与应用结算之间的状态变化；失效选择/模态/退出/恢复绝不读取配置或启动 HTTP。
use super::*;
use crate::projection_delivery::Inbox;
const ORIGIN: &str = "https://loom.example";
fn entry(accepted: bool) -> Arc<crate::projection_delivery::Invitation> {
    let value = crate::projection_delivery::tests::invitation(ORIGIN, accepted);
    Arc::new(
        Inbox::parse(
            &serde_json::to_vec(&serde_json::json!({"invitations":[value]})).unwrap(),
            ORIGIN,
        )
        .unwrap()
        .into_entries()
        .pop()
        .unwrap(),
    )
}
#[test]
fn same_id_replacement_accepted_and_cross_origin_candidates_cannot_reach_configuration() {
    let _com = Apartment::new().unwrap();
    unsafe {
        let (_class, mut app) = application();
        for mode in 0..3 {
            let current = entry(mode == 1);
            let selected = if mode == 0 {
                entry(false)
            } else {
                current.clone()
            };
            let mut view = app.authorization.borrow_mut();
            view.origin = Some(
                if mode == 2 {
                    "https://other.example"
                } else {
                    ORIGIN
                }
                .into(),
            );
            view.projection.entries = vec![current];
            view.request = Some(Request::Projection(ui::Request::Reject(selected)));
            drop(view);
            app.process_authorization_with_projection(|_| {
                panic!("stale selection read configuration")
            });
            assert_eq!(
                app.authorization.borrow().projection.status,
                "投送选择已失效，请刷新收件箱"
            );
            assert!(!app.remote_sessions.projection_pending());
        }
        app.shutdown();
    }
}
#[test]
fn duplicate_refresh_does_not_clear_an_active_request_or_candidates() {
    let _com = Apartment::new().unwrap();
    unsafe {
        let (_class, mut app) = application();
        let selected = entry(false);
        let mut view = app.authorization.borrow_mut();
        view.origin = Some(ORIGIN.into());
        view.projection.entries.push(selected.clone());
        view.projection.busy = true;
        view.request = Some(Request::Projection(ui::Request::Refresh));
        drop(view);
        app.process_authorization_with_projection(|_| panic!("duplicate read configuration"));
        assert!(app.authorization.borrow().projection.busy);
        assert!(Arc::ptr_eq(
            &app.authorization.borrow().projection.entries[0],
            &selected
        ));
        app.shutdown();
    }
}
#[test]
fn queued_inbox_request_cannot_cross_modal_exit_or_restore_start() {
    let _com = Apartment::new().unwrap();
    unsafe {
        let (_class, mut app) = application();
        app.authorization.borrow_mut().origin = Some(ORIGIN.into());
        for mode in 0..3 {
            authorization_ui::execute(
                app.documents[0].window.0,
                &app.documents[0].state,
                ui::REFRESH,
            )
            .unwrap();
            app.documents[0].state.file_dialog.set(mode == 0);
            app.exit_requested.set(mode == 1);
            if mode == 2 {
                app.request_restore_input(crate::desktop::session_job::Input::Bytes(
                    br#"{"documentSchemaVersion":1,"documentRevision":1,"otterNativeCoordinateSpace":{"version":1,"origin":[0,0],"scale":1},"stickers":[],"links":[]}"#.to_vec()), None, None).unwrap();
            }
            app.process_authorization_with_projection(|_| {
                panic!("blocked request read configuration")
            });
            assert!(!app.remote_sessions.projection_pending());
            assert!(!app.authorization.borrow().projection.busy);
            assert!(app
                .authorization
                .borrow()
                .projection
                .status
                .starts_with("投送操作未执行："));
            app.documents[0].state.file_dialog.set(false);
            app.exit_requested.set(false);
            app.cancel_restore();
            let end = Instant::now() + Duration::from_secs(10);
            while app.restore_job.is_some() {
                app.process_restore(false).unwrap();
                assert!(Instant::now() < end);
                std::thread::yield_now();
            }
        }
        app.shutdown();
    }
}

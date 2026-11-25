//! 首次普通投射经现有后台调度建立协作，不消费手动 Initialize 命令。
use super::*;
#[test]
fn first_projection_initializes_one_way_by_default_and_can_switch_two_way() {
    let _serial = serial();
    let _com = Apartment::new().unwrap();
    let (files, identity, _) = Files::new();
    let (listener, client, tls, pem) =
        crate::device_session_client::tests::fixture_with_host("127.0.0.1");
    let origin = client.origin().to_owned();
    std::fs::write(files.root().join("ca.pem"), pem).unwrap();
    let _peer = peer::Peer::start(listener, tls, files.root().to_path_buf());
    unsafe {
        let mut app = application(files.root());
        groups::authorize(&mut app, &files, identity, client);
        app.authorization.borrow_mut().targets.directory = Some(groups::directory(&origin, 1));
        mark(&app);
        send(&mut app, &files, 0);
        app.process_authorization_with_projection(|o| config(o, files.root()));
        app.projection_sender.updates.make_due();
        app.process_authorization_with_projection(|o| config(o, files.root()));
        assert!(
            app.projection_edit.busy(),
            "{}",
            app.authorization.borrow().editing.status
        );
        finish(&mut app, &files);
        assert_eq!(snapshot(&app).document().mode(), edit::Mode::OneWay);
        ui::execute(
            app.documents[0].window.0,
            &app.documents[0].state,
            ui::editing::TWO_WAY,
        )
        .unwrap();
        app.projection_edit_requests(&mut |o| config(o, files.root()));
        finish(&mut app, &files);
        assert_eq!(snapshot(&app).document().mode(), edit::Mode::TwoWay);
        assert!(!app.authorization.borrow().editing.recovery);
        app.shutdown();
    }
}

#[test]
fn two_way_selected_before_projection_reaches_the_real_collaboration_document() {
    let _serial = serial();
    let _com = Apartment::new().unwrap();
    let (files, identity, _) = Files::new();
    let (listener, client, tls, pem) =
        crate::device_session_client::tests::fixture_with_host("127.0.0.1");
    let origin = client.origin().to_owned();
    std::fs::write(files.root().join("ca.pem"), pem).unwrap();
    let _peer = peer::Peer::start(listener, tls, files.root().to_path_buf());
    unsafe {
        let mut app = application(files.root());
        groups::authorize(&mut app, &files, identity, client);
        app.authorization.borrow_mut().targets.directory = Some(groups::directory(&origin, 1));
        ui::editing::choose_mode(&app.documents[0].state, edit::Mode::TwoWay).unwrap();
        mark(&app);
        send(&mut app, &files, 0);
        app.process_authorization_with_projection(|o| config(o, files.root()));
        app.projection_sender.updates.make_due();
        app.process_authorization_with_projection(|o| config(o, files.root()));
        finish(&mut app, &files);
        // 初始化结束后由同一应用调度消费方向；自动内容暂停不能吞掉显式方向。
        let unit = app.documents[0].state.unit_id.clone();
        app.projection_edit
            .pause_automatic_for_test(&app.workspace, &unit);
        app.authorization.borrow_mut().busy = true;
        app.projection_edit_requests(&mut |o| config(o, files.root()));
        assert!(
            !app.documents[0]
                .state
                .projection_mode
                .borrow()
                .as_ref()
                .unwrap()
                .submitted
        );
        app.authorization.borrow_mut().busy = false;
        app.projection_edit_requests(&mut |_| {
            Err(crate::device_authorization_config::Error::Trust)
        });
        assert!(app.documents[0].state.projection_mode.borrow().is_none());
        assert_eq!(
            ui::editing::mode(&app.documents[0].state),
            edit::Mode::OneWay
        );
        ui::editing::choose_mode(&app.documents[0].state, edit::Mode::TwoWay).unwrap();
        app.projection_edit_requests(&mut |o| config(o, files.root()));
        finish(&mut app, &files);
        assert_eq!(snapshot(&app).document().mode(), edit::Mode::TwoWay);
        assert!(!app.authorization.borrow().editing.recovery);
        app.shutdown();
    }
}

#[test]
fn confirmed_targets_wait_for_ready_and_keep_checked_after_real_sender_checkpoint() {
    let _serial = serial();
    let _com = Apartment::new().unwrap();
    let (files, identity, _) = Files::new();
    let (listener, client, tls, pem) =
        crate::device_session_client::tests::fixture_with_host("127.0.0.1");
    let origin = client.origin().to_owned();
    std::fs::write(files.root().join("ca.pem"), pem).unwrap();
    let _peer = peer::Peer::start(listener, tls, files.root().to_path_buf());
    unsafe {
        let mut app = application(files.root());
        groups::authorize(&mut app, &files, identity, client);
        app.authorization.borrow_mut().targets.directory = Some(groups::directory(&origin, 2));
        mark(&app);
        app.process_authorization_with_projection(|o| config(o, files.root()));
        let owner = app.documents[0].window.0;
        let mut popup =
            crate::desktop::pin_settings::ConfirmedTargets::all(owner, &app.documents[0].state);
        app.authorization.borrow_mut().sender.ready = false;
        for _ in 0..3 {
            assert!(popup.poll(owner, &app.documents[0].state));
            assert!(app.authorization.borrow().request.is_none());
        }
        let end = Instant::now() + Duration::from_secs(15);
        loop {
            app.process_authorization_with_projection(|o| config(o, files.root()));
            settle(&mut app);
            if !popup.poll(owner, &app.documents[0].state) {
                break;
            }
            assert!(
                Instant::now() < end,
                "{}",
                app.authorization.borrow().sender.status
            );
            std::thread::yield_now();
        }
        assert_eq!(
            crate::desktop::pin_settings::ConfirmedTargets::checked(&app.documents[0].state),
            [true, true]
        );
        let mut reopened =
            crate::desktop::pin_settings::ConfirmedTargets::all(owner, &app.documents[0].state);
        assert!(!reopened.poll(owner, &app.documents[0].state));
        assert!(app.authorization.borrow().request.is_none());
        drop(reopened);
        drop(popup);
        app.shutdown();
    }
}

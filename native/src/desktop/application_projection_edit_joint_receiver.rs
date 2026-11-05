//! Receiver原接纳/真实NOACTIVATE帧/自动Apply/one_way冲突和默认保存重开消费者。
use super::*;
#[test]
#[ignore = "isolated two real Looms plus two native Application consumers"]
fn native_edit_application_receiver_automatic_readonly_resolution_and_reopen() {
    let _serial = serial();
    let _com = Apartment::new().unwrap();
    let (f, _client, _store) = fixture::Fixture::load();
    unsafe {
        let (_class, mut app) = application(&f.profile);
        app.projection_profile
            .start(f.profile.clone(), app.execution_wake.callback());
        until(&mut app, &f, |app| {
            matches!(
                app.projection_profile,
                crate::desktop::application::projection::profile::Owner::Ready { .. }
            )
        });
        renew(&mut app, &f);
        let focus = focus::Watch::new();
        command(&mut app, &f, ui::policy::CONFIRM);
        until(&mut app, &f, |app| {
            app.authorization.borrow().consent.stamp().is_some()
        });
        command(&mut app, &f, ui::projection::REFRESH);
        until(&mut app, &f, |app| {
            app.authorization.borrow().projection.entries.len() == 1
        });
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
        focus.assert_passive();
        let row = super::super::receiver::received(&app).unwrap();
        let unit = row["id"].as_str().unwrap().to_owned();
        let doc = app
            .documents
            .iter()
            .find(|d| d.state.unit_id.as_str() == unit)
            .unwrap();
        assert!(
            IsWindowVisible(doc.window.0).as_bool()
                && doc.state.canvas.borrow().confirmed_paints > 0
        );
        f.publish(
            "displayed.json",
            &json!({"binding":row["qrProjection"],"realFrame":true,"receiverNeverActivated":true}),
        );
        f.wait("initialized");
        until(&mut app, &f, |app| has_revision(app, 2) && quiet(app));
        f.publish(
            "initialized.json",
            &json!({"document":verify(&app,2,edit::Mode::TwoWay,4.0)}),
        );
        f.wait("source-edited");
        until(&mut app, &f, |app| has_revision(app, 3) && quiet(app));
        verify(&app, 3, edit::Mode::TwoWay, 6.0);
        shift(&app, 3.0);
        // 原receiver read释放原Job后自动Apply；无显式Submit，也不由fixture ack。
        until(&mut app, &f, |app| has_revision(app, 4) && quiet(app));
        acknowledged(&f, 4);
        let before = verify(&app, 4, edit::Mode::TwoWay, 9.0);
        reopen(&mut app, &f, true);
        assert_eq!(verify(&app, 4, edit::Mode::TwoWay, 9.0), before);
        assert_eq!(state(&app).unit_id.as_str(), unit);
        frame(&app, &f, "reopened.png");
        f.publish(
            "receiver-edited.json",
            &json!({"document":before,"reopened":true,"defaultCheckpoint":true}),
        );
        f.wait("one-way");
        until(&mut app, &f, |app| has_revision(app, 5) && quiet(app));
        verify(&app, 5, edit::Mode::OneWay, 9.0);
        shift(&app, 1.0);
        until(&mut app, &f, |app| {
            snapshot(app).conflict().is_some() && quiet(app)
        });
        let readonly = verify(&app, 5, edit::Mode::OneWay, 10.0);
        acknowledged(&f, 4);
        command(&mut app, &f, ui::editing::USE_REMOTE);
        until(&mut app, &f, |app| {
            snapshot(app).conflict().is_none() && quiet(app)
        });
        verify(&app, 5, edit::Mode::OneWay, 9.0);
        reopen(&mut app, &f, true);
        verify(&app, 5, edit::Mode::OneWay, 9.0);
        assert!(snapshot(&app).conflict().is_none());
        f.publish(
            "readonly.json",
            &json!({"document":readonly,"localPreserved":true,"remoteChosenAndReopened":true}),
        );
        f.wait("terminal");
        until(&mut app, &f, |app| {
            super::super::receiver::received(app)
                .is_some_and(|v| v["qrProjection"]["stopped"] == true)
                && quiet(app)
        });
        frame(&app, &f, "terminal.png");
        f.publish(
            "terminal.json",
            &json!({"document":verify(&app,5,edit::Mode::OneWay,9.0),"lastContentPreserved":true}),
        );
        focus.assert_passive();
        app.shutdown();
        drop(app);
        drain_messages();
    }
}

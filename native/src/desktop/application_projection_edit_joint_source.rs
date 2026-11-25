//! Source consumer through the original menu and 750ms schedule; the joint parent gate independently checks service authority instead of the test confirming its own journal.
use super::*;
#[test]
#[ignore = "isolated two real Looms plus two native Application consumers"]
fn native_edit_application_source_modes_automatic_checkpoint_and_reopen() {
    let _serial = serial();
    let _com = Apartment::new().unwrap();
    let (f, _client, _store) = fixture::Fixture::load();
    unsafe {
        let mut app = application(&f.profile);
        app.authorization.borrow_mut().consent.install_paused(true);
        renew(&mut app, &f);
        app.authorization.borrow_mut().request = Some(ui::Request::ProjectionTargets);
        until(&mut app, &f, |app| {
            app.authorization.borrow().targets.directory.is_some()
        });
        let index = app
            .authorization
            .borrow()
            .targets
            .directory
            .as_ref()
            .unwrap()
            .targets()
            .iter()
            .position(|v| v.id() == f.target)
            .unwrap();
        {
            let mut canvas = app.documents[0].state.canvas.borrow_mut();
            canvas.editor.set_style(
                crate::stroke_style::StrokeStyle::new(2.0, crate::stroke_style::Color(255, 0, 0))
                    .unwrap(),
            );
            canvas
                .editor
                .begin(crate::editor::Tool::Rectangle, Point { x: 4.0, y: 4.0 })
                .unwrap();
            canvas.editor.extend(Point { x: 12.0, y: 12.0 }).unwrap();
            assert!(canvas.editor.commit());
        }
        // Make the initial image genuinely canonical first, so a save immediately started by the fixture cannot stale the original send fence.
        app.start_checkpoint_with(&mut |_, _, message, warning| assert!(!warning, "{message}"));
        settle(&mut app);
        command(&mut app, &f, ui::sender::BASE + index);
        until(&mut app, &f, |app| {
            quiet(app)
                && app
                    .workspace_checkpoint
                    .as_ref()
                    .unwrap()
                    .sender_index()
                    .unwrap()
                    .entries()
                    .len()
                    == 1
        });
        let created = binding(&app);
        f.publish(
            "created.json",
            &json!({"envelope":created.envelope(),"digest":created.digest()}),
        );
        f.wait("accepted");
        // First projection automatically initializes collaboration through production scheduling, without injecting the old manual-initialization command.
        until(&mut app, &f, |app| has_revision(app, 1) && quiet(app));
        acknowledged(&f, 1);
        verify(&app, 1, edit::Mode::OneWay, 4.0);
        command(&mut app, &f, ui::editing::TWO_WAY);
        until(&mut app, &f, |app| has_revision(app, 2) && quiet(app));
        acknowledged(&f, 2);
        let document = verify(&app, 2, edit::Mode::TwoWay, 4.0);
        f.publish(
            "initialized.json",
            &json!({"document":document,"binding":binding(&app),"defaultCheckpoint":true}),
        );
        f.wait("receiver-initialized");
        shift(&app, 2.0);
        // Do not call start_edit directly; the original successful source probe automatically Applies local changes.
        until(&mut app, &f, |app| has_revision(app, 3) && quiet(app));
        acknowledged(&f, 3);
        f.publish(
            "source-edited.json",
            &json!({"document":verify(&app,3,edit::Mode::TwoWay,6.0)}),
        );
        f.wait("receiver-edited");
        until(&mut app, &f, |app| has_revision(app, 4) && quiet(app));
        acknowledged(&f, 4);
        let before = verify(&app, 4, edit::Mode::TwoWay, 9.0);
        reopen(&mut app, &f, false);
        assert_eq!(verify(&app, 4, edit::Mode::TwoWay, 9.0), before);
        acknowledged(&f, 4);
        frame(&app, &f, "reopened.png");
        f.publish(
            "reopened.json",
            &json!({"document":before,"reopened":true,"defaultCheckpoint":true}),
        );
        f.wait("switch-one-way");
        command(&mut app, &f, ui::editing::ONE_WAY);
        until(&mut app, &f, |app| has_revision(app, 5) && quiet(app));
        acknowledged(&f, 5);
        f.publish(
            "one-way.json",
            &json!({"document":verify(&app,5,edit::Mode::OneWay,9.0)}),
        );
        f.wait("receiver-readonly");
        command(&mut app, &f, ui::sender::STOP_BASE);
        until(&mut app, &f, |app| {
            quiet(app)
                && app
                    .workspace_checkpoint
                    .as_ref()
                    .unwrap()
                    .sender_index()
                    .unwrap()
                    .entries()
                    .is_empty()
        });
        f.publish("stopped.json", &json!({"stopped":true}));
        app.shutdown();
        drop(app);
        drain_messages();
    }
}

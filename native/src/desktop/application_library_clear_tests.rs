//! 清空走真实菜单命令、单槽 worker 和原 Receipt；仅内存夹具，不清理用户文件。
use super::*;
use crate::{desktop::reference_ui as ui, session_library::Kind};
#[test]
fn clear_commands_use_library_transaction_and_reject_cancelled_or_stale_work() {
    unsafe {
        let _apartment = crate::desktop::Apartment::new().unwrap();
        for command in [ui::CLEAR_RECYCLE, ui::CLEAR_REFERENCE] {
            for mode in 0..3 {
                let mut app = chrome::tests::app();
                let entries = serde_json::json!([{"entryId":"entry","sourceStickerId":"old","createdAt":"legacy","snapshot":{"id":"old","x":0,"y":0,"w":32,"h":32}}]);
                app.restored_session.as_mut().unwrap().original = serde_json::to_vec(&serde_json::json!({"documentRevision":9,"stickers":[],"links":[],"recycleBin":entries,"referenceLibrary":entries})).unwrap().into();
                app.sync_frozen_menu();
                let source = app.restored_session.as_ref().unwrap().original.clone();
                let doc = &app.documents[0];
                let hwnd = doc.window.0;
                crate::desktop::context_menu::execute(hwnd, &doc.state, command).unwrap();
                let exports = save_snapshot::payload_documents_for_test();
                app.process_reference(false).unwrap();
                assert_eq!(save_snapshot::payload_documents_for_test(), exports);
                assert!(app.save_job.as_ref().unwrap().1.is_library_update());
                assert!(app.take_save_candidate().is_err());
                if mode == 0 {
                    app.save_job.as_ref().unwrap().1.cancel();
                }
                if mode == 1 {
                    app.documents[0].state.canvas.borrow_mut().normal_opacity = 0.4;
                }
                wait(&app);
                let result = app.poll_reference();
                let current = &app.restored_session.as_ref().unwrap().original;
                if mode < 2 {
                    assert!(result.is_err());
                    assert!(current.same_storage(&source));
                } else {
                    result.unwrap();
                    let kind = if command == ui::CLEAR_RECYCLE {
                        Kind::RecycleBin
                    } else {
                        Kind::ReferenceLibrary
                    };
                    assert!(crate::session_library::decode(current, kind)
                        .unwrap()
                        .entries()
                        .is_empty());
                    assert_eq!(
                        crate::session_document::decode(current).unwrap().revision,
                        9
                    );
                    assert_eq!(app.documents.len(), 1);
                    assert!(IsWindow(Some(hwnd)).as_bool());
                }
                app.shutdown();
            }
        }
    }
}

#[test]
fn clearing_empty_or_missing_library_keeps_import_storage_and_saved_status() {
    unsafe {
        let _apartment = crate::desktop::Apartment::new().unwrap();
        let mut app = chrome::tests::app();
        let source = b"  {\"stickers\":[],\"links\":[],\"recycleBin\":[],\"documentRevision\":9}\n"
            .to_vec()
            .into();
        app.restored_session.as_mut().unwrap().original = source;
        let source = app.restored_session.as_ref().unwrap().original.clone();
        app.saved_state = Some(save_state::Stamp::capture(&app).unwrap());
        for command in [ui::CLEAR_RECYCLE, ui::CLEAR_REFERENCE] {
            app.sync_frozen_menu();
            let doc = &app.documents[0];
            crate::desktop::context_menu::execute(doc.window.0, &doc.state, command).unwrap();
            app.process_reference(false).unwrap();
            wait(&app);
            app.process_reference(false).unwrap();
            assert!(app.save_job.is_none());
            assert!(source.same_storage(&app.restored_session.as_ref().unwrap().original));
            assert_eq!(app.save_status(), save_state::Status::Unchanged);
            assert!(app.documents[0].is_alive());
        }
        app.shutdown();
    }
}

fn wait(app: &Application) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while !app.save_job.as_ref().unwrap().1.is_ready() {
        assert!(std::time::Instant::now() < deadline);
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
}

#[test]
fn insert_exports_and_encodes_only_its_target_but_remove_and_unstar_only_capture_the_receipt() {
    unsafe {
        let _apartment = crate::desktop::Apartment::new().unwrap();
        let image = crate::desktop::image::Image::from_encoded(
            crate::desktop::application::smoke::image_fixture::PNG,
        )
        .unwrap();
        let instance = crate::desktop::workspace_window_fixture::register();
        let mut app = Application::new(
            instance,
            DocumentWindow::create(instance, State::new(Some(image), false).unwrap(), false)
                .unwrap(),
        );
        let second = DocumentWindow::create(
            instance,
            State::new(
                Some(crate::desktop::image::Image {
                    width: 2,
                    height: 2,
                    pixels: [255, 0, 0, 255].repeat(4).into(),
                }),
                false,
            )
            .unwrap(),
            false,
        )
        .unwrap();
        let other = second.state.unit_id.clone();
        app.workspace.add_units([other.clone()]).unwrap();
        app.documents.push(second);
        let target = app.documents[0].state.unit_id.clone();
        let entry = serde_json::json!({"entryId":"entry","sourceStickerId":"old","createdAt":"legacy","snapshot":{"id":"old","x":0,"y":0,"w":32,"h":32}});
        app.restored_session.as_mut().unwrap().original = serde_json::to_vec(&serde_json::json!({
            "documentRevision":9,"stickers":[
                {"id":target.as_str(),"x":0,"y":0,"w":32,"h":32,"futureTarget":{"kept":true}},
                {"id":other.as_str(),"x":0,"y":0,"w":32,"h":32,"futureOther":{"kept":true}}
            ],"links":[],"recycleBin":[entry],"futureRoot":{"kept":true}
        }))
        .unwrap()
        .into();
        let source = app.restored_session.as_ref().unwrap().original.clone();
        let exports = save_snapshot::payload_documents_for_test();
        let remove = || crate::desktop::frozen_library_ui::Choice {
            remove: true,
            source: source.clone(),
            kind: Kind::RecycleBin,
            entry_id: "entry".into(),
        };
        // Removing a library entry still validates the current workspace viewport.
        let scale = app.documents[0].state.canvas.borrow().viewport.scale;
        app.documents[0].state.canvas.borrow_mut().viewport.scale = 0.0;
        assert!(app.request_frozen_remove(remove()).is_err());
        assert!(app.save_job.is_none());
        app.documents[0].state.canvas.borrow_mut().viewport.scale = scale;
        app.request_frozen_remove(remove()).unwrap();
        assert_eq!(save_snapshot::payload_documents_for_test(), exports);
        wait(&app);
        app.process_reference(false).unwrap();
        assert_eq!(entries(&app, Kind::RecycleBin), 0);

        // An unrelated window still participates in the receipt and can invalidate publication.
        let before_add = app.restored_session.as_ref().unwrap().original.clone();
        let doc = &app.documents[0];
        crate::desktop::context_menu::execute(doc.window.0, &doc.state, ui::SET).unwrap();
        app.process_reference(false).unwrap();
        app.documents[1].state.canvas.borrow_mut().normal_opacity = 0.4;
        wait(&app);
        assert!(app.process_reference(false).is_err());
        assert!(before_add.same_storage(&app.restored_session.as_ref().unwrap().original));

        // Only the full-save encoder should see this deliberately unencodable unrelated window.
        app.documents[1].state.canvas.borrow_mut().normal_opacity = 2.0;
        let (_, whole) = app.capture_session_snapshot().unwrap();
        assert!(matches!(
            whole.encode(&std::sync::atomic::AtomicBool::new(false)),
            Err(crate::session_write::Error::Window)
        ));

        for (command, expected_entries) in [(ui::SET, 1), (ui::REMOVE, 0)] {
            let exports = save_snapshot::payload_documents_for_test();
            let doc = &app.documents[0];
            crate::desktop::context_menu::execute(doc.window.0, &doc.state, command).unwrap();
            app.process_reference(false).unwrap();
            assert_eq!(
                save_snapshot::payload_documents_for_test() - exports,
                expected_entries
            );
            wait(&app);
            app.process_reference(false).unwrap();
            assert_eq!(entries(&app, Kind::ReferenceLibrary), expected_entries);
            if expected_entries != 0 {
                let current = &app.restored_session.as_ref().unwrap().original;
                let value: serde_json::Value = serde_json::from_slice(current).unwrap();
                let frozen = &value["referenceLibrary"][0]["snapshot"];
                assert_eq!(frozen["id"], target.as_str());
                assert_eq!(frozen["futureTarget"]["kept"], true);
                assert!(frozen["src"]
                    .as_str()
                    .unwrap()
                    .starts_with("data:image/png;base64,"));
                assert_eq!(value["stickers"][1]["futureOther"]["kept"], true);
                assert_eq!(value["futureRoot"]["kept"], true);
            }
        }
        let record = app.restored_session.as_ref().unwrap();
        assert_eq!(
            crate::session_document::decode(&record.original)
                .unwrap()
                .revision,
            9
        );
        assert_eq!(app.documents.len(), 2);
        assert!(app.documents.iter().all(|doc| doc.is_alive()));
        app.shutdown();
    }
}

fn entries(app: &Application, kind: Kind) -> usize {
    let source = &app.restored_session.as_ref().unwrap().original;
    crate::session_library::decode(source, kind)
        .unwrap()
        .entries()
        .len()
}

//! 原菜单删除协调器、Receipt 和 HWND；只删除内存合成记录。
use super::*;
use crate::session_library::Kind;

#[test]
fn frozen_gallery_retirement_outlives_menu_and_reaps_after_its_only_notification() {
    use crate::desktop::loom_execution_tasks::reap_fixture::{Gate, WAIT};

    unsafe {
        let _apartment = crate::desktop::Apartment::new().unwrap();
        let (mut app, _) = fixture(2);
        let pump = execution_test_pump::Pump::new();
        let wake = app.execution_wake.callback();
        let gate = Gate::new(move || wake());
        let mut workers = app.frozen_library.borrow().workers.clone();
        workers.notify = Some(gate.notify);
        popup::test_close_index_menu(workers.clone());
        gate.noticed.recv_timeout(WAIT).unwrap();
        assert!(PeekMessageW(
            &mut MSG::default(),
            None,
            WM_APP + 92,
            WM_APP + 92,
            PM_REMOVE
        )
        .as_bool());
        app.pending_windows(false).unwrap();
        assert!(workers.deadline().is_some());
        while pump.step(&mut app) != WM_NULL {}
        assert!(workers.deadline().is_some());
        gate.release.send(()).unwrap();
        while workers.deadline().is_some() {
            pump.step(&mut app);
        }
        assert!(gate.noticed.try_recv().is_err());
    }
}

unsafe fn fixture(count: usize) -> (Application, ui::Pending) {
    let app = chrome::tests::app();
    let entries: Vec<_> = (0..count).map(|n| serde_json::json!({
        "entryId": format!("entry-{n}"), "sourceStickerId": format!("source-{n}"),
        "createdAt": format!("row-{n}"), "snapshot": {"id":format!("source-{n}"),"x":0,"y":0,"w":32,"h":32}
    })).collect();
    let mut app = app;
    app.restored_session.as_mut().unwrap().original = serde_json::to_vec(&serde_json::json!({
        "stickers":[],"links":[],"documentRevision":9,"recycleBin":entries,"referenceLibrary":entries
    })).unwrap().into();
    app.sync_frozen_menu();
    let doc = &app.documents[0];
    let pending = ui::Pending {
        origin: doc.state.unit_id.clone(),
        target: popup::target::MenuTarget::capture(&doc.state.canvas.borrow()),
        point: POINT::default(),
    };
    (app, pending)
}
#[test]
fn both_libraries_delete_repeatedly_in_the_open_panel_and_keep_scroll_until_empty() {
    unsafe {
        let _apartment = crate::desktop::Apartment::new().unwrap();
        for count in [3, 65] {
            for library in 0..2 {
                let (mut app, pending) = fixture(count);
                let hwnd = app.documents[0].window.0;
                let flag = app.documents[0].state.file_dialog.clone();
                let guard = crate::desktop::file_dialog::Guard::begin(&flag).unwrap();
                let source = app.restored_session.as_ref().unwrap().original.clone();
                let mut removal = Removal::default();
                popup::test_continuous_removal(hwnd, source, library, count, &mut |choice| {
                    let result = removal.step(&mut app, &pending, &flag, choice);
                    assert!(flag.get());
                    result
                });
                let source = &app.restored_session.as_ref().unwrap().original;
                for (slot, kind) in [Kind::RecycleBin, Kind::ReferenceLibrary]
                    .into_iter()
                    .enumerate()
                {
                    assert_eq!(
                        crate::session_library::decode(source, kind)
                            .unwrap()
                            .entries()
                            .len(),
                        if slot == library { count - 3 } else { count }
                    );
                }
                assert_eq!(crate::session_document::decode(source).unwrap().revision, 9);
                assert_eq!(app.documents.len(), 1);
                assert!(app.documents[0].is_alive());
                assert!(app.save_job.is_none());
                drop(guard);
                app.shutdown();
            }
        }
    }
}
#[test]
fn modal_cancelled_and_stale_removals_keep_source_and_restore_menu_gate() {
    unsafe {
        let _apartment = crate::desktop::Apartment::new().unwrap();
        for mode in 0..4 {
            let (mut app, pending) = fixture(2);
            let flag = app.documents[0].state.file_dialog.clone();
            let guard = crate::desktop::file_dialog::Guard::begin(&flag).unwrap();
            let source = app.restored_session.as_ref().unwrap().original.clone();
            let other_modal = std::rc::Rc::new(Cell::new(mode == 0));
            app.modals.register(&other_modal);
            let mut removal = Removal::default();
            let result = removal.step(
                &mut app,
                &pending,
                &flag,
                Some(crate::desktop::frozen_library_ui::Choice {
                    remove: true,
                    source: source.clone(),
                    kind: Kind::RecycleBin,
                    entry_id: "entry-0".into(),
                }),
            );
            assert!(flag.get());
            if mode == 0 {
                assert!(result.is_err());
            } else {
                result.unwrap();
                if mode == 1 {
                    app.save_job.as_ref().unwrap().1.cancel();
                }
                if mode == 2 {
                    app.documents[0].state.canvas.borrow_mut().normal_opacity = 0.4;
                }
                if mode == 3 {
                    app.documents[0].state.canvas.borrow_mut().crop_mode = true;
                }
                let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
                while !app.save_job.as_ref().unwrap().1.is_ready() {
                    assert!(std::time::Instant::now() < deadline);
                    std::thread::sleep(std::time::Duration::from_millis(1));
                }
                assert!(removal.step(&mut app, &pending, &flag, None).is_err());
                assert!(flag.get());
            }
            assert!(app
                .restored_session
                .as_ref()
                .unwrap()
                .original
                .same_storage(&source));
            drop(guard);
            app.shutdown();
        }
    }
}
#[test]
fn context_message_queues_once_without_entering_a_nested_menu() {
    unsafe {
        let _apartment = crate::desktop::Apartment::new().unwrap();
        let (mut app, _) = fixture(2);
        let doc = &app.documents[0];
        SendMessageW(
            doc.window.0,
            WM_CONTEXTMENU,
            None,
            Some(LPARAM(40 | (60 << 16))),
        );
        assert!(!doc.state.file_dialog.get());
        let pending = app.frozen_library.borrow_mut().menu.take().unwrap();
        assert_eq!((pending.point.x, pending.point.y), (40, 60));
        assert!(current(&app, &pending));
        app.documents[0].state.canvas.borrow_mut().crop_mode = true;
        assert!(!current(&app, &pending));
        app.show_context_menu(&pending).unwrap();
        app.shutdown();
    }
}

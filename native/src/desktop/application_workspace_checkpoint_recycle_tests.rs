//! Recycle-bin checkpoint deduplication regressions across last-sticker closure, restoration, manual-file isolation, restart, and write failure.
use super::*;
use crate::session_library::Kind;

fn drop_probe(source: &Source) -> Box<dyn Fn() -> bool> {
    match source {
        Source::Imported(bytes) => {
            let weak = std::sync::Arc::downgrade(bytes);
            Box::new(move || weak.strong_count() == 0)
        }
        Source::Encoded(bytes) => {
            let weak = bytes.downgrade();
            Box::new(move || weak.strong_count() == 0)
        }
    }
}

fn assert_single_resident_source(app: &Application) {
    let owner = app.workspace_checkpoint.as_ref().unwrap();
    let canonical = owner
        .original
        .as_ref()
        .expect("committed checkpoint source");
    let (_, stamped) = owner.stamp.as_ref().expect("committed checkpoint stamp");
    let retained = &app.restored_session.as_ref().unwrap().original;
    let menu = app.frozen_library.borrow();
    assert!(canonical.same_storage(stamped));
    assert!(canonical.same_storage(retained));
    assert!(menu
        .source
        .as_ref()
        .is_some_and(|source| canonical.same_storage(source)));
    assert!(app
        .authorization
        .borrow()
        .sender
        .stop_source
        .as_ref()
        .is_none_or(|source| canonical.same_storage(source)));
}

unsafe fn settle_all(app: &mut Application) {
    let end = Instant::now() + Duration::from_secs(10);
    loop {
        app.pending_windows(false).unwrap();
        app.pending_workspace_save_with(|_, _, message, warning| assert!(!warning, "{message}"));
        if app.save_job.is_none() && !app.recycle_maintenance_pending {
            break;
        }
        assert!(Instant::now() < end, "recycle checkpoint settle deadline");
        std::thread::sleep(Duration::from_millis(1));
    }
}

unsafe fn recycle_active(app: &mut Application) {
    app.sync_frozen_menu();
    let doc = app
        .documents
        .iter()
        .find(|doc| doc.state.canvas.borrow().image.is_some())
        .expect("active image sticker");
    crate::desktop::context_menu::execute(
        doc.window.0,
        &doc.state,
        crate::desktop::reference_ui::RECYCLE,
    )
    .unwrap();
    app.process_reference(false).unwrap();
    ready(app);
    app.pending_windows(false).unwrap();
}

unsafe fn restore_recycle_entry(app: &mut Application, index: usize) {
    let source = app.restored_session.as_ref().unwrap().original.clone();
    let entry_id = crate::session_library::decode(&source, Kind::RecycleBin)
        .unwrap()
        .entries()[index]
        .entry_id
        .to_string();
    app.request_frozen(source, Kind::RecycleBin, entry_id, POINT { x: 24, y: 24 })
        .unwrap();
    let end = Instant::now() + Duration::from_secs(10);
    while app.restore_job.is_some() {
        app.pending_windows(false).unwrap();
        assert!(Instant::now() < end, "recycle restore deadline");
        std::thread::sleep(Duration::from_millis(1));
    }
}

#[test]
fn closing_last_sticker_and_restoring_from_recycle_bin_keep_single_resident_source() {
    unsafe {
        run(|instance| {
            // Mode A: close two stickers into recycleBin down to the final empty standby host, then restore one and close the last pin via WM_CLOSE.
            let directory = Directory::new();
            let mut app = new_app(instance);
            let image = crate::desktop::image::Image::from_encoded(
                crate::desktop::application::smoke::image_fixture::PNG,
            )
            .unwrap();
            app.add_state(State::new(Some(image), false).unwrap(), false)
                .unwrap();
            app.documents[0].state.canvas.borrow_mut().normal_opacity = 0.75;
            app.install_workspace_checkpoint(Some(Profile::open(&directory.0).unwrap()), true)
                .unwrap();
            settle(&mut app);

            // Close first sticker via RECYCLE while one active sticker remains.
            recycle_active(&mut app);
            let interim_first = drop_probe(&app.restored_session.as_ref().unwrap().original);
            settle(&mut app);
            assert!(interim_first(), "interim recycle buffer must be released");
            assert_single_resident_source(&app);

            // Close the final active sticker via RECYCLE -> empty standby host.
            recycle_active(&mut app);
            assert_eq!(app.documents.len(), 1);
            assert!(app
                .restored_session
                .as_ref()
                .unwrap()
                .transient_host
                .is_some());
            let interim_last = drop_probe(&app.restored_session.as_ref().unwrap().original);
            settle(&mut app);
            assert!(
                interim_last(),
                "last-sticker recycle buffer must be released after checkpoint"
            );
            assert_single_resident_source(&app);
            assert!(app.session_checkpoint_current());
            let disk_bytes = std::fs::read(directory.path()).unwrap();
            let disk = crate::session_document::decode(&disk_bytes).unwrap();
            assert!(disk.windows.is_empty());
            assert_eq!(disk.library(Kind::RecycleBin).unwrap().entries().len(), 2);

            // Restore the first recycled sticker (opacity 0.75) on the empty standby host; 1 entry remains in recycleBin.
            let pre_restore = drop_probe(&app.restored_session.as_ref().unwrap().original);
            restore_recycle_entry(&mut app, 0);
            assert_eq!(app.documents.len(), 2);
            let post_append = drop_probe(&app.restored_session.as_ref().unwrap().original);
            settle(&mut app);
            assert!(pre_restore());
            assert!(post_append());
            assert_single_resident_source(&app);
            assert_eq!(app.documents[1].state.canvas.borrow().normal_opacity, 0.75);

            // Mode B: close the remaining active sticker and host via WM_CLOSE -> resume_empty_with -> settle.
            let pre_wm_close = drop_probe(&app.restored_session.as_ref().unwrap().original);
            let handles: Vec<HWND> = app.documents.iter().map(|doc| doc.window.0).collect();
            for hwnd in handles {
                SendMessageW(hwnd, WM_CLOSE, None, None);
            }
            assert!(drain_messages());
            app.resume_empty_with(|_| Ok(())).unwrap();
            settle(&mut app);
            assert!(
                pre_wm_close(),
                "pre-WM_CLOSE Source with deleted sticker must be released"
            );
            assert_single_resident_source(&app);
            let after_close_bytes = std::fs::read(directory.path()).unwrap();
            let after_close = crate::session_document::decode(&after_close_bytes).unwrap();
            assert!(after_close.windows.is_empty());
            assert_eq!(
                after_close
                    .library(Kind::RecycleBin)
                    .unwrap()
                    .entries()
                    .len(),
                1
            );
            drop(app);
        });
    }
}

#[test]
fn persistence_restart_and_manual_save_isolation_preserve_revisions_and_single_source() {
    unsafe {
        run(|instance| {
            let directory = Directory::new();
            let mut app = new_app(instance);
            app.documents[0].state.canvas.borrow_mut().normal_opacity = 0.65;
            app.install_workspace_checkpoint(Some(Profile::open(&directory.0).unwrap()), true)
                .unwrap();
            settle(&mut app);
            recycle_active(&mut app);
            settle(&mut app);
            let persisted_bytes = std::fs::read(directory.path()).unwrap();
            let persisted_rev = crate::session_document::decode(&persisted_bytes)
                .unwrap()
                .revision;
            assert_single_resident_source(&app);
            settle(&mut app);
            assert_eq!(
                std::fs::read(directory.path()).unwrap(),
                persisted_bytes,
                "settled single-source checkpoint must not rewrite unchanged workspace"
            );
            drop(app);

            // Restart from workspace.json with empty stickers and 1 recycleBin item.
            let mut restarted = new_app(instance);
            restarted
                .install_workspace_checkpoint(Some(Profile::open(&directory.0).unwrap()), true)
                .unwrap();
            assert_single_resident_source(&restarted);
            assert!(restarted
                .restored_session
                .as_ref()
                .unwrap()
                .transient_host
                .is_some());
            restore_recycle_entry(&mut restarted, 0);
            let append_retained =
                drop_probe(&restarted.restored_session.as_ref().unwrap().original);
            settle_all(&mut restarted);
            assert!(
                append_retained(),
                "restoring the final recycleBin entry must also release the append buffer"
            );
            assert_single_resident_source(&restarted);
            let after_restart_bytes = std::fs::read(directory.path()).unwrap();
            let after_restart_restore =
                crate::session_document::decode(&after_restart_bytes).unwrap();
            assert_eq!(after_restart_restore.revision, persisted_rev + 1);
            assert_eq!(after_restart_restore.windows.len(), 1);
            assert!(after_restart_restore
                .library(Kind::RecycleBin)
                .unwrap()
                .entries()
                .is_empty());

            // Manual Save-As baseline isolation: checkpoint must never overwrite manual file baseline or break Save.
            let manual = directory.0.join("manual.json");
            crate::desktop::application::frozen_menu::reference::smoke::save(
                &mut restarted,
                Some(manual.clone()),
            )
            .unwrap();
            let manual_initial_bytes = std::fs::read(&manual).unwrap();
            let manual_rev = crate::session_document::decode(&manual_initial_bytes)
                .unwrap()
                .revision;
            recycle_active(&mut restarted);
            let manual_recycle_source = restarted
                .restored_session
                .as_ref()
                .unwrap()
                .original
                .clone();
            settle_all(&mut restarted);
            assert!(
                manual_recycle_source
                    .same_storage(&restarted.restored_session.as_ref().unwrap().original),
                "manual saved_path sessions must retain independent baseline revision"
            );
            restarted.request_save_existing().unwrap();
            ready(&restarted);
            restarted
                .take_save_candidate()
                .unwrap()
                .unwrap()
                .commit_ready(&mut restarted, &AtomicBool::new(false))
                .unwrap();
            let manual_saved_bytes = std::fs::read(&manual).unwrap();
            let manual_saved = crate::session_document::decode(&manual_saved_bytes).unwrap();
            assert_eq!(manual_saved.revision, manual_rev + 1);
            assert_eq!(
                manual_saved
                    .library(Kind::RecycleBin)
                    .unwrap()
                    .entries()
                    .len(),
                1
            );
            drop(restarted);
        });
    }
}

#[test]
fn write_failure_and_deferred_fence_preserve_recycle_bin_recovery_and_reconcile_when_idle() {
    unsafe {
        run(|instance| {
            let directory = Directory::new();
            let mut app = new_app(instance);
            app.install_workspace_checkpoint(Some(Profile::open(&directory.0).unwrap()), true)
                .unwrap();
            settle(&mut app);

            // Deferred fence: hold configure_recycle during checkpoint commit, then clear and verify reconciliation.
            recycle_active(&mut app);
            let fenced_old = drop_probe(&app.restored_session.as_ref().unwrap().original);
            let unit = app.documents[0].state.unit_id.clone();
            app.frozen_library.borrow_mut().configure_recycle = Some(unit);
            app.start_checkpoint().unwrap();
            ready(&app);
            app.poll_checkpoint().unwrap();
            assert!(
                !fenced_old(),
                "active frozen-library fence must keep pre-checkpoint Source alive"
            );
            app.frozen_library.borrow_mut().configure_recycle = None;
            settle(&mut app);
            assert!(
                fenced_old(),
                "clearing transient fence must reconcile to canonical checkpoint Source"
            );
            assert_single_resident_source(&app);

            // Restore and recycle again while workspace.json has an external conflict -> checkpoint stops without losing recycleBin.
            restore_recycle_entry(&mut app, 0);
            settle(&mut app);
            std::fs::write(directory.path(), b"external conflict").unwrap();
            recycle_active(&mut app);
            let surviving = app.restored_session.as_ref().unwrap().original.clone();
            let mut warnings = 0usize;
            let end = Instant::now() + Duration::from_secs(10);
            while !app.workspace_checkpoint.as_ref().unwrap().stopped {
                app.pending_workspace_save_with(|_, _, _, warning| {
                    warnings += usize::from(warning);
                });
                assert!(Instant::now() < end, "write failure stop deadline");
                std::thread::sleep(Duration::from_millis(1));
            }
            assert_eq!(warnings, 1);
            assert_eq!(
                std::fs::read(directory.path()).unwrap(),
                b"external conflict"
            );
            assert!(surviving.same_storage(&app.restored_session.as_ref().unwrap().original));
            assert!(app
                .frozen_library
                .borrow()
                .source
                .as_ref()
                .is_some_and(|s| surviving.same_storage(s)));

            // Even after checkpoint write failure, user can restore from recycleBin in memory and Save As to a new path.
            restore_recycle_entry(&mut app, 0);
            assert_eq!(app.documents.len(), 2);
            let rescued = directory.0.join("rescued.json");
            app.request_save_new(rescued.clone()).unwrap();
            ready(&app);
            app.take_save_candidate()
                .unwrap()
                .unwrap()
                .commit_ready(&mut app, &AtomicBool::new(false))
                .unwrap();
            let rescued_bytes = std::fs::read(&rescued).unwrap();
            let rescued_doc = crate::session_document::decode(&rescued_bytes).unwrap();
            assert_eq!(rescued_doc.windows.len(), 1);
            drop(app);
        });
    }
}

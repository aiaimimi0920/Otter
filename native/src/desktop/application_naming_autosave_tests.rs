//! 自动保存只通过生产调度器写合成配置；不使用常驻计时器或真实用户目录。
use super::*;
#[path = "application_cache_save_tests.rs"]
mod cache_tests;
#[path = "application_naming_profile_tests.rs"]
mod profile_tests;
unsafe fn toggle(app: &Application) {
    let doc = &app.documents[0];
    crate::desktop::application::execute(
        doc.window.0,
        &doc.state,
        crate::desktop::application::AUTO_NAMING,
    )
    .unwrap();
}
unsafe fn tick(app: &mut Application) -> Result<Option<String>> {
    app.pending_naming_save_auto_with(|_, _| panic!("Autosave must not open file dialog"))
}
unsafe fn settle(app: &mut Application) -> Result<()> {
    let deadline = Instant::now() + Duration::from_secs(5);
    tick(app)?;
    while app.naming_save_job.is_some() {
        tick(app)?;
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(1));
    }
    Ok(())
}
#[test]
fn opt_in_saves_changed_settings_once_and_new_windows_share_preference() {
    unsafe {
        let _environment = environment();
        let files = Files::new();
        let mut app = app();
        load(&mut app, &files);
        app.naming.borrow_mut().sticker_save_pattern = "automatic".into();
        tick(&mut app).unwrap();
        assert!(app.naming_save_job.is_none());
        assert_eq!(fs::read(files.source()).unwrap(), BASE);
        toggle(&app);
        app.documents[0].state.file_dialog.set(true);
        tick(&mut app).unwrap();
        assert!(app.naming_save_job.is_none());
        app.documents[0].state.file_dialog.set(false);
        settle(&mut app).unwrap();
        assert!(app.naming_autosave.get());
        let saved = fs::read(files.source()).unwrap();
        assert_ne!(saved, BASE);
        for _ in 0..20 {
            tick(&mut app).unwrap();
            assert!(app.naming_save_job.is_none());
        }
        assert_eq!(fs::read(files.source()).unwrap(), saved);
        let doc = &app.documents[0];
        crate::desktop::application::execute(
            doc.window.0,
            &doc.state,
            crate::desktop::application::NEW_WINDOW,
        )
        .unwrap();
        app.pending_windows(false).unwrap();
        assert!(std::rc::Rc::ptr_eq(
            &app.documents[1].state.copy_authority,
            &app.copy_authority
        ));
        assert!(std::rc::Rc::ptr_eq(
            &app.documents[1].state.naming_autosave,
            &app.naming_autosave
        ));
        assert!(app.documents[1].state.naming_autosave.get());
    }
}
#[test]
fn disabling_discards_pending_automatic_result_but_manual_save_still_works() {
    unsafe {
        let _environment = environment();
        let files = Files::new();
        let mut app = app();
        load(&mut app, &files);
        app.naming.borrow_mut().sticker_save_pattern = "pending".into();
        toggle(&app);
        tick(&mut app).unwrap();
        assert!(app.naming_save_job.is_some());
        toggle(&app);
        settle(&mut app).unwrap();
        assert_eq!(fs::read(files.source()).unwrap(), BASE);
        begin(&mut app).unwrap();
        finish_save(&mut app).unwrap();
        assert_ne!(fs::read(files.source()).unwrap(), BASE);
    }
}
#[test]
fn conflicts_stop_retrying_and_cancel_pending_exit_without_losing_runtime_settings() {
    unsafe {
        let _environment = environment();
        for external in [true, false] {
            let files = Files::new();
            let mut app = app();
            load(&mut app, &files);
            if external {
                fs::write(files.source(), b"external").unwrap();
            }
            app.naming.borrow_mut().sticker_save_pattern = "pending".into();
            toggle(&app);
            tick(&mut app).unwrap();
            if !external {
                app.naming.borrow_mut().sticker_save_pattern = "newer".into();
            }
            app.exit_requested.set(true);
            assert!(settle(&mut app).is_err());
            assert!(!app.naming_autosave.get());
            assert!(!app.exit_requested.get());
            assert_eq!(
                app.naming.borrow().sticker_save_pattern,
                if external { "pending" } else { "newer" }
            );
            for _ in 0..5 {
                tick(&mut app).unwrap();
                assert!(app.naming_save_job.is_none());
            }
            assert_eq!(
                fs::read(files.source()).unwrap(),
                if external {
                    b"external".as_slice()
                } else {
                    BASE
                }
            );
        }
    }
}
#[test]
fn no_source_is_rejected_and_application_exit_waits_for_enabled_save() {
    unsafe {
        let _environment = environment();
        let files = Files::new();
        let mut app = app();
        toggle(&app);
        assert!(tick(&mut app).is_err());
        assert!(!app.naming_autosave.get());
        load(&mut app, &files);
        let original = app.naming_source.clone().unwrap();
        let recovered = crate::settings_source::Source::recovered(
            files.target(),
            files.source(),
            BASE.to_vec(),
        )
        .unwrap();
        app.install_startup_settings(recovered).unwrap();
        toggle(&app);
        assert!(tick(&mut app).is_err());
        assert!(!app.naming_autosave.get());
        app.install_startup_settings(original).unwrap();
        app.naming.borrow_mut().sticker_save_pattern = "before-exit".into();
        toggle(&app);
        app.exit_requested.set(true);
        assert!(!app.close_requested_with(|_| panic!("Workspace exit must wait for settings")));
        assert!(app.exit_requested.get());
        settle(&mut app).unwrap();
        assert!(app.exit_requested.get());
        assert!(!app.naming_autosave_blocks_exit());
        assert_ne!(fs::read(files.source()).unwrap(), BASE);
    }
}

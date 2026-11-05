//! 显式覆盖菜单：使用独立合成配置，验证基线推进、外部冲突及失败保留。
use super::*;
#[path = "application_naming_autosave_tests.rs"]
mod autosave_tests;
const BASE: &[u8] = br#"{"schemaVersion":1,"extension":{"keep":17},"fileNaming":{"unknown":true}}"#;
unsafe fn load(app: &mut Application, files: &Files) {
    fs::write(files.source(), BASE).unwrap();
    queue(app);
    app.pending_naming_import_with(|_, _| Ok(Some(files.source())))
        .unwrap();
    finish(app).unwrap();
}
unsafe fn queue_update(app: &Application) {
    let doc = &app.documents[0];
    crate::desktop::application::execute(
        doc.window.0,
        &doc.state,
        crate::desktop::application::UPDATE_NAMING,
    )
    .unwrap();
}
unsafe fn begin(app: &mut Application) -> Result<Option<String>> {
    queue_update(app);
    app.pending_naming_save_with(|_, _| panic!("Updating loaded file must not open save-as dialog"))
}

#[test]
fn repeated_updates_advance_exact_baseline_and_preserve_extensions() {
    unsafe {
        let _environment = environment();
        let files = Files::new();
        let mut app = app();
        load(&mut app, &files);
        for pattern in ["第一_{shortId}", "第二_{date}"] {
            app.naming.borrow_mut().sticker_save_pattern = pattern.into();
            begin(&mut app).unwrap();
            finish_save(&mut app).unwrap();
            let bytes = fs::read(files.source()).unwrap();
            let source = app.naming_source.as_ref().unwrap();
            assert_eq!(source.bytes().as_ref(), bytes);
            assert!(std::sync::Arc::ptr_eq(
                source.document(),
                &app.naming_document
            ));
            let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
            assert_eq!(value["schemaVersion"], 1);
            assert_eq!(value["extension"]["keep"], 17);
            assert_eq!(value["fileNaming"]["unknown"], true);
            assert_eq!(value["fileNaming"]["stickerSavePattern"], pattern);
            assert_eq!(fs::read_dir(&files.0).unwrap().count(), 1);
        }
    }
}

#[test]
fn external_edits_are_not_overwritten_and_loaded_baseline_is_not_advanced() {
    unsafe {
        let _environment = environment();
        let files = Files::new();
        let mut app = app();
        load(&mut app, &files);
        let before = app.naming_source.clone().unwrap();
        fs::write(files.source(), b"external writer").unwrap();
        app.naming.borrow_mut().sticker_save_pattern = "new".into();
        begin(&mut app).unwrap();
        assert!(finish_save(&mut app).is_err());
        assert_eq!(fs::read(files.source()).unwrap(), b"external writer");
        assert!(std::sync::Arc::ptr_eq(
            app.naming_source.as_ref().unwrap(),
            &before
        ));
        assert_eq!(fs::read_dir(&files.0).unwrap().count(), 1);
    }
}

#[test]
fn stale_results_and_drop_preserve_original_file_without_recovery_debris() {
    unsafe {
        let _environment = environment();
        for case in 0..3 {
            let files = Files::new();
            let mut app = app();
            load(&mut app, &files);
            app.naming.borrow_mut().sticker_save_pattern = "pending".into();
            begin(&mut app).unwrap();
            match case {
                0 => {
                    app.naming.borrow_mut().sticker_save_pattern = "newer".into();
                    assert!(finish_save(&mut app).is_err());
                }
                1 => {
                    app.naming_document = Default::default();
                    assert!(finish_save(&mut app).is_err());
                }
                _ => {}
            }
            drop(app);
            assert_eq!(fs::read(files.source()).unwrap(), BASE);
            assert_eq!(fs::read_dir(&files.0).unwrap().count(), 1);
        }
    }
}

#[test]
fn updating_without_loaded_source_is_rejected_not_redirected() {
    unsafe {
        let _environment = environment();
        let mut app = app();
        assert!(begin(&mut app).is_err());
        assert!(app.naming_save_job.is_none());
    }
}

#[test]
fn save_as_becomes_active_source_and_next_update_leaves_imported_file_unchanged() {
    unsafe {
        let _environment = environment();
        let files = Files::new();
        let mut app = app();
        load(&mut app, &files);
        queue_save(&app);
        app.pending_naming_save_with(|_, _| Ok(Some(files.target())))
            .unwrap();
        finish_save(&mut app).unwrap();
        assert_eq!(app.naming_source.as_ref().unwrap().path(), files.target());
        app.naming.borrow_mut().sticker_save_pattern = "new-active".into();
        begin(&mut app).unwrap();
        finish_save(&mut app).unwrap();
        assert_eq!(fs::read(files.source()).unwrap(), BASE);
        let value: serde_json::Value =
            serde_json::from_slice(&fs::read(files.target()).unwrap()).unwrap();
        assert_eq!(value["fileNaming"]["stickerSavePattern"], "new-active");
    }
}

#[test]
fn recovered_source_requires_save_as_before_it_can_be_updated() {
    unsafe {
        let _environment = environment();
        let files = Files::new();
        fs::write(files.source(), BASE).unwrap();
        let recovered = crate::settings_source::Source::recovered(
            files.target(),
            files.source(),
            BASE.to_vec(),
        )
        .unwrap();
        assert!(crate::settings_update::Prepared::prepare(
            &recovered,
            Settings::default(),
            &std::sync::atomic::AtomicBool::new(false)
        )
        .is_err());
        let mut app = app();
        app.install_startup_settings(recovered).unwrap();
        assert!(begin(&mut app).is_err());
        assert!(app.naming_save_job.is_none());
        queue_save(&app);
        app.pending_naming_save_with(|_, _| Ok(Some(files.target())))
            .unwrap();
        finish_save(&mut app).unwrap();
        assert!(app
            .naming_source
            .as_ref()
            .unwrap()
            .recovery_target()
            .is_none());
        app.naming.borrow_mut().sticker_save_pattern = "after-recovery".into();
        begin(&mut app).unwrap();
        finish_save(&mut app).unwrap();
        assert_eq!(fs::read(files.source()).unwrap(), BASE);
        let value: serde_json::Value =
            serde_json::from_slice(&fs::read(files.target()).unwrap()).unwrap();
        assert_eq!(value["fileNaming"]["stickerSavePattern"], "after-recovery");
    }
}

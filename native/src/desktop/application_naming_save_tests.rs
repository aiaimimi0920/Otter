//! 共享隐藏宿主夹具，验证真实设置另存为和源配置保留。
use super::*;
#[path = "application_naming_update_tests.rs"]
mod update_tests;
struct Files(PathBuf);
impl Files {
    fn new() -> Self {
        let root = PathBuf::from("C:/Users/Public/nas_home/AI/GameEditor/linshi");
        fs::create_dir_all(&root).unwrap();
        let id = unsafe { windows::Win32::System::Com::CoCreateGuid().unwrap() };
        let path = root.join(format!("otter-settings-save-{id:?}"));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn target(&self) -> PathBuf {
        self.0.join("新设置.json")
    }
    fn source(&self) -> PathBuf {
        self.0.join("原设置.json")
    }
}
impl Drop for Files {
    fn drop(&mut self) {
        for path in [self.target(), self.source()] {
            if path.exists() {
                fs::remove_file(path).unwrap();
            }
        }
        // 非递归删除同时验证暂存文件无残留。
        fs::remove_dir(&self.0).unwrap();
    }
}
unsafe fn queue_save(app: &Application) {
    let doc = &app.documents[0];
    crate::desktop::application::execute(
        doc.window.0,
        &doc.state,
        crate::desktop::application::SAVE_NAMING,
    )
    .unwrap();
}
unsafe fn finish_save(app: &mut Application) -> Result<()> {
    let deadline = Instant::now() + Duration::from_secs(5);
    while app.naming_save_job.is_some() {
        app.pending_naming_save_with(|_, _| panic!("unexpected save dialog"))?;
        assert!(Instant::now() < deadline, "naming save deadline");
        std::thread::sleep(Duration::from_millis(1));
    }
    Ok(())
}

#[test]
fn imported_extensions_survive_save_and_reimport_without_source_changes() {
    unsafe {
        let _environment = environment();
        let files = Files::new();
        let bytes = br#"{"schemaVersion":1,"translationProviderMode":"gateway","extension":[1,true],"fileNaming":{"future":{"x":17}}}"#;
        fs::write(files.source(), bytes).unwrap();
        let mut app = app();
        queue(&app);
        app.pending_naming_import_with(|_, _| Ok(Some(files.source())))
            .unwrap();
        finish(&mut app).unwrap();
        app.naming.borrow_mut().sticker_save_pattern = "保存_{shortId}".into();
        queue_save(&app);
        app.pending_naming_save_with(|_, _| Ok(Some(files.target())))
            .unwrap();
        assert!(!files.target().exists());
        finish_save(&mut app).unwrap();
        let saved = fs::read(files.target()).unwrap();
        assert!(!saved.starts_with(&[0xef, 0xbb, 0xbf]));
        let value: serde_json::Value = serde_json::from_slice(&saved).unwrap();
        assert_eq!(value["schemaVersion"], 1);
        assert_eq!(value["translationProviderMode"], "gateway");
        assert_eq!(value["extension"], serde_json::json!([1, true]));
        assert_eq!(value["fileNaming"]["future"]["x"], 17);
        assert_eq!(value["fileNaming"]["stickerSavePattern"], "保存_{shortId}");
        assert_eq!(fs::read(files.source()).unwrap(), bytes);
        app.naming.borrow_mut().sticker_save_pattern = "changed".into();
        queue(&app);
        app.pending_naming_import_with(|_, _| Ok(Some(files.target())))
            .unwrap();
        finish(&mut app).unwrap();
        assert_eq!(app.naming.borrow().sticker_save_pattern, "保存_{shortId}");
    }
}

#[test]
fn cancel_modal_and_stale_selection_never_start_disk_save() {
    unsafe {
        let _environment = environment();
        let mut app = app();
        queue_save(&app);
        queue_save(&app);
        app.documents[0].state.file_dialog.set(true);
        app.pending_naming_save_with(|_, _| panic!("modal must defer"))
            .unwrap();
        app.documents[0].state.file_dialog.set(false);
        app.pending_naming_save_with(|_, _| Ok(None)).unwrap();
        app.pending_naming_save_with(|_, _| panic!("coalesced request delivered twice"))
            .unwrap();
        queue_save(&app);
        assert!(app
            .pending_naming_save_with(|_, state| {
                state.naming.borrow_mut().sticker_save_pattern = "newer".into();
                Ok(Some(PathBuf::from("C:/not-created.json")))
            })
            .is_err());
        assert!(app.naming_save_job.is_none());
    }
}

#[test]
fn late_changes_closed_owner_and_competing_writer_prevent_publication() {
    unsafe {
        let _environment = environment();
        for case in 0..4 {
            let files = Files::new();
            let mut app = app();
            queue_save(&app);
            app.pending_naming_save_with(|_, _| Ok(Some(files.target())))
                .unwrap();
            match case {
                0 => app.naming.borrow_mut().sticker_save_pattern = "newer".into(),
                1 => app.naming_document = Default::default(),
                2 => {
                    SendMessageW(app.documents[0].window.0, WM_CLOSE, None, None);
                }
                _ => fs::write(files.target(), b"competing writer").unwrap(),
            }
            assert!(finish_save(&mut app).is_err());
            assert!(app.naming_save_job.is_none());
            if case == 3 {
                assert_eq!(fs::read(files.target()).unwrap(), b"competing writer");
            } else {
                assert!(!files.target().exists());
            }
        }
    }
}

#[test]
fn existing_target_is_rejected_and_application_drop_cleans_pending_save() {
    unsafe {
        let _environment = environment();
        let files = Files::new();
        fs::write(files.source(), b"keep").unwrap();
        let mut app = app();
        queue_save(&app);
        assert!(app
            .pending_naming_save_with(|_, _| Ok(Some(files.source())))
            .is_err());
        assert!(app.naming_save_job.is_none());
        queue_save(&app);
        app.pending_naming_save_with(|_, _| Ok(Some(files.target())))
            .unwrap();
        drop(app);
        assert!(!files.target().exists());
        assert_eq!(fs::read(files.source()).unwrap(), b"keep");
    }
}

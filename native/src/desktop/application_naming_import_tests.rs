//! 隐藏 HWND + 真实后台文件读取；仅路径选择注入，不弹出文件框。
use super::*;
#[path = "application_art_issuer_tests.rs"]
mod art_issuer_tests;
#[path = "application_local_art_tests.rs"]
mod local_art_tests;
#[path = "application_naming_save_tests.rs"]
mod save_tests;
use std::{
    fs,
    io::Write,
    time::{Duration, Instant},
};

struct Fixture(PathBuf);
struct Class(HINSTANCE);
impl Drop for Class {
    fn drop(&mut self) {
        unsafe {
            let _ = UnregisterClassW(w!("Otter.Native.Workspace"), Some(self.0));
        }
    }
}
struct Environment {
    _class: Class,
    _apartment: crate::desktop::Apartment,
    _serial: std::sync::MutexGuard<'static, ()>,
}
unsafe fn environment() -> Environment {
    let serial = crate::loom_bridge_client::tests::execution_job::SERIAL
        .lock()
        .unwrap();
    let apartment = crate::desktop::Apartment::new().unwrap();
    let instance: HINSTANCE = windows::Win32::System::LibraryLoader::GetModuleHandleW(None)
        .unwrap()
        .into();
    assert_ne!(
        RegisterClassW(&WNDCLASSW {
            hInstance: instance,
            lpszClassName: w!("Otter.Native.Workspace"),
            lpfnWndProc: Some(crate::desktop::procedure::window_proc),
            ..Default::default()
        }),
        0
    );
    Environment {
        _class: Class(instance),
        _apartment: apartment,
        _serial: serial,
    }
}
impl Fixture {
    fn new(bytes: &[u8]) -> Self {
        let root = PathBuf::from("C:/Users/Public/nas_home/AI/GameEditor/linshi");
        fs::create_dir_all(&root).unwrap();
        let id = unsafe { windows::Win32::System::Com::CoCreateGuid().unwrap() };
        let path = root.join(format!("otter-naming-{id:?}.json"));
        fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .unwrap()
            .write_all(bytes)
            .unwrap();
        Self(path)
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_file(&self.0).unwrap();
    }
}
unsafe fn app() -> Application {
    let instance = HINSTANCE(
        windows::Win32::System::LibraryLoader::GetModuleHandleW(None)
            .unwrap()
            .0,
    );
    let first = DocumentWindow::create(instance, State::new(None, false).unwrap(), false).unwrap();
    Application::new(instance, first)
}
unsafe fn queue(app: &Application) {
    let doc = &app.documents[0];
    super::super::commands::execute(
        doc.window.0,
        &doc.state,
        super::super::commands::IMPORT_NAMING,
    )
    .unwrap();
}
unsafe fn finish(app: &mut Application) -> Result<()> {
    let deadline = Instant::now() + Duration::from_secs(5);
    while app.naming_import_job.is_some() {
        app.pending_naming_import_with(|_, _| panic!("unexpected dialog"))?;
        assert!(Instant::now() < deadline, "naming import deadline");
        std::thread::sleep(Duration::from_millis(1));
    }
    Ok(())
}

#[test]
fn invalid_recycle_or_newer_policy_does_not_partially_install_import() {
    unsafe {
        let _environment = environment();
        for stale in [false, true] {
            let bytes = if stale {
                br#"{"fileNaming":{"stickerSavePattern":"must-not-install"},"cache":{"recycleBinMaxEntries":2}}"#.as_slice()
            } else {
                br#"{"fileNaming":{"stickerSavePattern":"must-not-install"},"cache":{"recycleBinMaxEntries":501}}"#.as_slice()
            };
            let fixture = Fixture::new(bytes);
            let mut app = app();
            let before = app.naming_document.clone();
            queue(&app);
            app.pending_naming_import_with(|_, _| Ok(Some(fixture.0.clone())))
                .unwrap();
            if stale {
                app.recycle_policy.recycle_bin_max_entries = 3;
            }
            let expected = app.recycle_policy;
            assert!(finish(&mut app).is_err());
            assert_eq!(*app.naming.borrow(), Settings::default());
            assert_eq!(app.recycle_policy, expected);
            assert!(std::sync::Arc::ptr_eq(&before, &app.naming_document));
            assert!(app.naming_source.is_none());
            assert!(app.naming_import_job.is_none());
            assert_eq!(fs::read(&fixture.0).unwrap(), bytes);
        }
    }
}

#[test]
fn menu_cancel_coalesces_and_stale_selection_preserves_settings() {
    unsafe {
        let _environment = environment();
        let mut app = app();
        queue(&app);
        queue(&app);
        app.documents[0].state.file_dialog.set(true);
        app.pending_naming_import_with(|_, _| panic!("modal must defer selection"))
            .unwrap();
        assert!(app.documents[0].state.naming_import_requested.get());
        app.documents[0].state.file_dialog.set(false);
        app.pending_naming_import_with(|_, _| Ok(None)).unwrap();
        app.pending_naming_import_with(|_, _| panic!("duplicate request"))
            .unwrap();
        assert!(app.naming_import_job.is_none());
        queue(&app);
        assert!(app
            .pending_naming_import_with(|_, state| {
                state.naming.borrow_mut().sticker_save_pattern = "changed".into();
                Ok(Some(PathBuf::from("C:/not-opened.json")))
            })
            .is_err());
        assert_eq!(app.naming.borrow().sticker_save_pattern, "changed");
        assert!(app.naming_import_job.is_none());
    }
}

#[test]
fn real_worker_imports_without_modifying_source_and_releases_handle() {
    unsafe {
        let _environment = environment();
        let bytes = br#"{"schemaVersion":2,"fileNaming":{"stickerSavePattern":"imported_{shortId}"},"other":17}"#;
        let fixture = Fixture::new(bytes);
        let mut app = app();
        queue(&app);
        app.pending_naming_import_with(|_, _| Ok(Some(fixture.0.clone())))
            .unwrap();
        queue(&app);
        app.pending_naming_import_with(|_, _| panic!("busy request must coalesce"))
            .unwrap();
        finish(&mut app).unwrap();
        assert_eq!(
            app.naming.borrow().sticker_save_pattern,
            "imported_{shortId}"
        );
        assert_eq!(
            app.documents[0].state.naming.borrow().sticker_save_pattern,
            "imported_{shortId}"
        );
        assert_eq!(fs::read(&fixture.0).unwrap(), bytes);
        drop(fixture);
    }
}

#[test]
fn closed_origin_and_oversized_file_are_rejected() {
    unsafe {
        let _environment = environment();
        for oversized in [false, true] {
            let bytes = if oversized {
                vec![b' '; crate::app_settings_document::MAX_BYTES + 1]
            } else {
                br#"{"fileNaming":{"stickerSavePattern":"must-not-install"}}"#.to_vec()
            };
            let fixture = Fixture::new(&bytes);
            let mut app = app();
            queue(&app);
            app.pending_naming_import_with(|_, _| Ok(Some(fixture.0.clone())))
                .unwrap();
            if !oversized {
                SendMessageW(app.documents[0].window.0, WM_CLOSE, None, None);
            }
            assert!(finish(&mut app).is_err());
            assert_eq!(*app.naming.borrow(), Settings::default());
            assert!(app.naming_import_job.is_none());
            assert_eq!(fs::read(&fixture.0).unwrap(), bytes);
        }
    }
}

#[test]
fn invalid_file_and_late_result_do_not_replace_current_settings() {
    unsafe {
        let _environment = environment();
        for bytes in [
            br#"{"schemaVersion":3}"#.as_slice(),
            br#"{"fileNaming":{"stickerSavePattern":"imported"}}"#,
        ] {
            let fixture = Fixture::new(bytes);
            let mut app = app();
            queue(&app);
            app.pending_naming_import_with(|_, _| Ok(Some(fixture.0.clone())))
                .unwrap();
            app.naming.borrow_mut().sticker_save_pattern = "newer".into();
            assert!(finish(&mut app).is_err());
            assert!(app.naming_import_job.is_none());
            assert_eq!(app.naming.borrow().sticker_save_pattern, "newer");
            assert_eq!(fs::read(&fixture.0).unwrap(), bytes);
        }
    }
}

#[test]
fn replacement_window_with_same_unit_id_cannot_receive_old_import() {
    unsafe {
        let _environment = environment();
        let fixture = Fixture::new(br#"{"fileNaming":{"stickerSavePattern":"old-owner"}}"#);
        let mut app = app();
        queue(&app);
        app.pending_naming_import_with(|_, _| Ok(Some(fixture.0.clone())))
            .unwrap();
        let mut state = State::new(None, false).unwrap();
        state.unit_id = app.documents[0].state.unit_id.clone();
        state.naming = app.naming.clone();
        let replacement = DocumentWindow::create(app.instance, state, false).unwrap();
        app.documents[0] = replacement;
        assert!(finish(&mut app).is_err());
        assert_eq!(*app.naming.borrow(), Settings::default());
    }
}

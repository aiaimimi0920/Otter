//! 首次 profile 保存仍走不覆盖发布；测试目录显式隔离且按已知路径清理。
use super::*;
struct Profile(PathBuf);
impl Profile {
    fn target(&self) -> PathBuf {
        self.0.join("app-settings.json")
    }
}
impl Drop for Profile {
    fn drop(&mut self) {
        if self.target().exists() {
            fs::remove_file(self.target()).unwrap();
        }
        if self.0.exists() {
            fs::remove_dir(&self.0).unwrap();
        }
    }
}
#[test]
fn first_manual_save_creates_profile_and_later_update_advances_it() {
    unsafe {
        let _environment = environment();
        let files = Files::new();
        let profile = Profile(files.0.join("new-profile"));
        let mut app = app();
        app.install_profile_target(Some(profile.target()));
        assert!(!profile.0.exists());
        app.naming.borrow_mut().sticker_save_pattern = "first".into();
        begin(&mut app).unwrap();
        finish_save(&mut app).unwrap();
        assert_eq!(app.naming_source.as_ref().unwrap().path(), profile.target());
        app.naming.borrow_mut().sticker_save_pattern = "second".into();
        begin(&mut app).unwrap();
        finish_save(&mut app).unwrap();
        let bytes = fs::read(profile.target()).unwrap();
        let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(value["fileNaming"]["stickerSavePattern"], "second");
        assert_eq!(value["schemaVersion"], 2);
    }
}
#[test]
fn opted_in_autosave_creates_only_after_change_and_preserves_concurrent_first_writer() {
    unsafe {
        let _environment = environment();
        for conflict in [false, true] {
            let files = Files::new();
            let profile = Profile(files.0.join("new-profile"));
            let mut app = app();
            app.install_profile_target(Some(profile.target()));
            toggle(&app);
            tick(&mut app).unwrap();
            assert!(!profile.0.exists());
            if conflict {
                fs::create_dir(&profile.0).unwrap();
                fs::write(profile.target(), b"other writer").unwrap();
            }
            app.naming.borrow_mut().sticker_save_pattern = "automatic-profile".into();
            let result = settle(&mut app);
            if conflict {
                assert!(result.is_err());
                assert!(!app.naming_autosave.get());
                assert_eq!(fs::read(profile.target()).unwrap(), b"other writer");
                assert!(app.naming_source.is_none());
            } else {
                result.unwrap();
                assert!(app.naming_source.is_some());
                assert!(profile.target().exists());
            }
        }
    }
}

//! Profile-aware language startup reads only the explicitly isolated profile and never migrates it.
use crate::{desktop::settings_startup, display_language::DisplayLanguage, ui_text};
use std::{fs, path::PathBuf};

struct Profile(PathBuf);
impl Profile {
    fn new() -> Self {
        let id = unsafe { windows::Win32::System::Com::CoCreateGuid().unwrap() };
        let path = crate::test_temp_root().join(format!("otter-language-startup-{id:?}"));
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }
    fn settings(&self) -> PathBuf {
        self.0.join("app-settings.json")
    }
}
impl Drop for Profile {
    fn drop(&mut self) {
        if self.settings().exists() {
            fs::remove_file(self.settings()).unwrap();
        }
        fs::remove_dir(&self.0).unwrap();
    }
}

#[test]
fn isolated_profile_loads_english_and_legacy_chinese_without_rewriting_settings() {
    let profile = Profile::new();
    let loaded = settings_startup::resolve_in_profile(None, false, Some(&profile.0)).unwrap();
    assert!(loaded.source.is_none());
    loaded.install_language().unwrap();
    assert_eq!(ui_text::current_language(), DisplayLanguage::English);
    assert!(!profile.settings().exists());
    for (bytes, expected) in [
        (
            br#"{"schemaVersion":2,"displayLanguage":"en"}"#.as_slice(),
            DisplayLanguage::English,
        ),
        (
            br#"{"schemaVersion":2}"#.as_slice(),
            DisplayLanguage::SimplifiedChinese,
        ),
        (
            br#"{"schemaVersion":2,"displayLanguage":"zh-CN"}"#.as_slice(),
            DisplayLanguage::SimplifiedChinese,
        ),
    ] {
        fs::write(profile.settings(), bytes).unwrap();
        let loaded = settings_startup::resolve_in_profile(None, false, Some(&profile.0)).unwrap();
        assert_eq!(loaded.source.as_ref().unwrap().path(), profile.settings());
        loaded.install_language().unwrap();
        assert_eq!(ui_text::current_language(), expected);
        assert_eq!(fs::read(profile.settings()).unwrap(), bytes);
    }
    fs::write(
        profile.settings(),
        br#"{"schemaVersion":2,"displayLanguage":"fr"}"#,
    )
    .unwrap();
    assert!(settings_startup::resolve_in_profile(None, false, Some(&profile.0)).is_err());
    assert_eq!(
        ui_text::current_language(),
        DisplayLanguage::SimplifiedChinese
    );
}

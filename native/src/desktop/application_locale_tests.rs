//! Hidden owners and real isolated settings files verify both entrances without changing the user's profile.
use super::*;
use crate::{app_settings_document::Document, settings_source::Source};
use std::{fs, path::PathBuf, time::Duration};
#[path = "application_locale_lifecycle_tests.rs"]
mod lifecycle;
#[path = "settings_startup_language_tests.rs"]
mod startup_language;

struct Fixture(PathBuf);
impl Fixture {
    fn new(bytes: Option<&[u8]>) -> Self {
        let root = PathBuf::from("C:/Users/Public/nas_home/AI/GameEditor/linshi");
        fs::create_dir_all(&root).unwrap();
        let id = unsafe { windows::Win32::System::Com::CoCreateGuid().unwrap() };
        let path = root.join(format!("otter-display-language-{id:?}.json"));
        if let Some(bytes) = bytes {
            fs::write(&path, bytes).unwrap();
        }
        Self(path)
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        if self.0.exists() {
            fs::remove_file(&self.0).unwrap();
        }
    }
}
unsafe fn application(fixture: &Fixture) -> Application {
    let instance = crate::desktop::workspace_window_fixture::register();
    let state = State::new(None, false).unwrap();
    let document = DocumentWindow::create(instance, state, false).unwrap();
    let mut app = Application::new(instance, document);
    if fixture.0.exists() {
        let source = Source::new(fixture.0.clone(), fs::read(&fixture.0).unwrap()).unwrap();
        app.install_startup_settings(source).unwrap();
    } else {
        app.install_profile_target(Some(fixture.0.clone()));
    }
    crate::ui_text::set_language(app.display_language_status().language);
    app
}
fn request(id: &str, language: DisplayLanguage, revision: u64) -> RemoteRequest {
    RemoteRequest {
        request_id: id.into(),
        source: "test-loom|https://127.0.0.1|generation-1".into(),
        language,
        expected_revision: revision,
        expires_at: Instant::now() + Duration::from_secs(10),
        cancel: Arc::new(AtomicBool::new(false)),
    }
}
unsafe fn finish(app: &mut Application) {
    let until = Instant::now() + Duration::from_secs(10);
    while app.locale_blocks_exit() {
        app.process_display_language();
        assert!(Instant::now() < until, "language owner did not settle");
        std::thread::sleep(Duration::from_millis(1));
    }
}

#[test]
fn local_and_remote_choices_share_persistence_and_old_retries_do_not_overwrite() {
    unsafe {
        let _apartment = crate::desktop::Apartment::new().unwrap();
        let fixture = Fixture::new(Some(
            br#"{"schemaVersion":2,"translationTargetLanguage":"en","unknown":17}"#,
        ));
        let mut app = application(&fixture);
        assert_eq!(
            app.display_language_status().language,
            DisplayLanguage::SimplifiedChinese
        );
        app.request_display_language(request("remote-1", DisplayLanguage::English, 0))
            .unwrap();
        finish(&mut app);
        let receipt = app.take_display_language_result().unwrap();
        assert_eq!(receipt.outcome, Outcome::Applied);
        assert_eq!(receipt.revision, 1);
        assert_eq!(crate::ui_text::current_language(), DisplayLanguage::English);
        app.documents[0]
            .state
            .display_language_request
            .set(Some(DisplayLanguage::SimplifiedChinese));
        finish(&mut app);
        assert_eq!(app.display_language_status().revision, 2);
        app.request_display_language(request("remote-1", DisplayLanguage::English, 0))
            .unwrap();
        let receipt = app.take_display_language_result().unwrap();
        assert_eq!(receipt.outcome, Outcome::Applied);
        assert_eq!(receipt.revision, 1);
        assert_eq!(app.display_language_status().revision, 2);
        assert_eq!(
            crate::ui_text::current_language(),
            DisplayLanguage::SimplifiedChinese
        );
        let persisted = Document::decode(&fs::read(&fixture.0).unwrap()).unwrap();
        assert_eq!(
            persisted.display_language().unwrap(),
            DisplayLanguage::SimplifiedChinese
        );
        assert_eq!(persisted.translation_options().0, "en");
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&persisted.encode().unwrap()).unwrap()
                ["unknown"],
            17
        );
    }
}

#[test]
fn modal_defers_language_save_and_invalidated_remote_never_writes() {
    unsafe {
        let _apartment = crate::desktop::Apartment::new().unwrap();
        let original = br#"{"schemaVersion":2}"#;
        let fixture = Fixture::new(Some(original));
        let mut app = application(&fixture);
        let request = request("remote-modal", DisplayLanguage::English, 0);
        let cancel = request.cancel.clone();
        app.documents[0].state.file_dialog.set(true);
        app.request_display_language(request).unwrap();
        app.process_display_language();
        assert!(!app.locale_busy());
        assert_eq!(fs::read(&fixture.0).unwrap(), original);
        cancel.store(true, Ordering::Release);
        app.documents[0].state.file_dialog.set(false);
        finish(&mut app);
        assert_eq!(
            app.take_display_language_result().unwrap().outcome,
            Outcome::Expired
        );
        assert_eq!(fs::read(&fixture.0).unwrap(), original);
        assert_eq!(
            crate::ui_text::current_language(),
            DisplayLanguage::SimplifiedChinese
        );
    }
}

#[test]
fn revision_conflict_and_disk_conflict_do_not_publish_language() {
    unsafe {
        let _apartment = crate::desktop::Apartment::new().unwrap();
        let fixture = Fixture::new(Some(br#"{"schemaVersion":2}"#));
        let mut app = application(&fixture);
        app.request_display_language(request("stale", DisplayLanguage::English, 1))
            .unwrap();
        assert_eq!(
            app.take_display_language_result().unwrap().outcome,
            Outcome::Conflict
        );
        fs::write(&fixture.0, br#"{"schemaVersion":2,"external":true}"#).unwrap();
        app.request_display_language(request("conflict", DisplayLanguage::English, 0))
            .unwrap();
        finish(&mut app);
        assert!(matches!(
            app.take_display_language_result().unwrap().outcome,
            Outcome::Failed(_)
        ));
        assert_eq!(app.display_language_status().revision, 0);
        assert_eq!(
            crate::ui_text::current_language(),
            DisplayLanguage::SimplifiedChinese
        );
        assert!(String::from_utf8(fs::read(&fixture.0).unwrap())
            .unwrap()
            .contains("external"));
    }
}

#[test]
fn first_profile_is_created_and_same_language_request_still_advances_revision() {
    unsafe {
        let _apartment = crate::desktop::Apartment::new().unwrap();
        let fixture = Fixture::new(None);
        let mut app = application(&fixture);
        assert_eq!(
            app.display_language_status().language,
            DisplayLanguage::English
        );
        app.request_display_language(request("first", DisplayLanguage::English, 0))
            .unwrap();
        finish(&mut app);
        assert_eq!(
            app.take_display_language_result().unwrap().outcome,
            Outcome::Applied
        );
        let saved = Document::decode(&fs::read(&fixture.0).unwrap()).unwrap();
        assert_eq!(saved.display_language().unwrap(), DisplayLanguage::English);
        assert_eq!(saved.display_language_revision().unwrap(), 1);
    }
}

//! Read-only startup configuration: explicit paths and the same bounded file admission; return errors before any screenshot or window creation.
use crate::app_settings_document::Document;
use std::{
    path::Path,
    sync::{atomic::AtomicBool, Arc},
};
use windows::{
    core::{Error, Result},
    Win32::Foundation::{E_FAIL, E_INVALIDARG},
};

pub(super) struct Loaded {
    pub source: Option<Arc<crate::settings_source::Source>>,
    pub default_target: Option<std::path::PathBuf>,
}
impl Loaded {
    pub(super) fn install_language(&self) -> Result<()> {
        let language = match &self.source {
            Some(source) => source
                .document()
                .display_language()
                .map_err(|error| Error::new(E_INVALIDARG, error))?,
            None => crate::display_language::DisplayLanguage::English,
        };
        crate::ui_text::set_language(language);
        Ok(())
    }
    pub(super) fn install(&self, state: &super::State) -> Result<()> {
        if let Some(source) = &self.source {
            let policy = source
                .document()
                .cache_policy()
                .map_err(|error| Error::new(E_INVALIDARG, error))?;
            *state.naming.borrow_mut() = source.document().naming().clone();
            state.cache_policy.set(policy);
        }
        Ok(())
    }
}
pub(super) fn resolve(explicit: Option<&Path>, hidden_check: bool) -> Result<Loaded> {
    resolve_in_profile(explicit, hidden_check, None)
}
/// Independent Tile processes read the selected profile once; they do not become settings writers.
pub(super) fn resolve_in_profile(
    explicit: Option<&Path>,
    hidden_check: bool,
    profile: Option<&Path>,
) -> Result<Loaded> {
    let override_root = std::env::var_os("OTTER_NATIVE_PROFILE_DIR");
    let local = std::env::var_os("LOCALAPPDATA");
    let selection = crate::settings_profile::select(
        explicit,
        profile.map(Path::as_os_str).or(override_root.as_deref()),
        local.as_deref(),
        hidden_check,
    )
    .map_err(|e| Error::new(E_INVALIDARG, e))?;
    let Some(selection) = selection else {
        return Ok(Loaded {
            source: None,
            default_target: None,
        });
    };
    if selection.allow_missing {
        crate::profile_directory::validate(
            selection
                .path
                .parent()
                .expect("profile filename has parent"),
        )
        .map_err(|e| Error::new(E_INVALIDARG, e.to_string()))?;
    }
    let source = load(&selection.path, selection.allow_missing)?;
    if let Some(source) = &source {
        source.document().recycle_policy().map_err(|error| {
            Error::new(E_INVALIDARG, format!("Invalid recycle settings: {error}"))
        })?;
        source.document().cache_policy().map_err(|error| {
            Error::new(E_INVALIDARG, format!("Invalid cache settings: {error}"))
        })?;
    }
    if hidden_check && selection.allow_missing {
        println!(
            "native-settings-profile: configured=true loaded={}",
            source.is_some()
        );
    }
    Ok(Loaded {
        source,
        default_target: selection.allow_missing.then_some(selection.path),
    })
}
fn load(path: &Path, allow_missing: bool) -> Result<Option<Arc<crate::settings_source::Source>>> {
    let result = super::session_job::file::read_recovering_limited(
        path,
        &AtomicBool::new(false),
        crate::app_settings_document::MAX_BYTES,
    );
    let (bytes, recovery) = match result {
        Ok(result) => result,
        Err(error) if allow_missing && error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(None)
        }
        Err(error) => return Err(Error::new(E_FAIL, format!("Cannot read settings: {error}"))),
    };
    let source = match recovery {
        Some(recovery) => {
            crate::settings_source::Source::recovered(path.to_path_buf(), recovery, bytes)
        }
        None => crate::settings_source::Source::new(path.to_path_buf(), bytes),
    };
    source
        .map(Some)
        .map_err(|e| Error::new(E_INVALIDARG, format!("Invalid settings: {e}")))
}
pub(super) fn verify(state: &super::State, document: &Document) -> Result<()> {
    let policy = document
        .cache_policy()
        .map_err(|error| Error::new(E_INVALIDARG, format!("Invalid cache settings: {error}")))?;
    if state.cache_policy.get() != policy {
        return Err(Error::new(
            E_FAIL,
            "Startup cache settings were not installed",
        ));
    }
    if *state.naming.borrow() != *document.naming() {
        return Err(Error::new(
            E_FAIL,
            "Startup naming settings were not installed",
        ));
    }
    Ok(())
}
pub(super) unsafe fn announce_recovery(
    hwnd: windows::Win32::Foundation::HWND,
    source: &crate::settings_source::Source,
    hidden: bool,
) {
    let Some(target) = source.recovery_target() else {
        return;
    };
    if hidden {
        println!("native-settings-recovery: recovered=true update-blocked=true readonly=true");
    } else {
        use windows::{core::HSTRING, Win32::UI::WindowsAndMessaging::*};
        let message = crate::ui_text::format(
            "action-settings-recovery",
            &[
                ("target", target.display().to_string().into()),
                ("recovery", source.path().display().to_string().into()),
            ],
        );
        MessageBoxW(
            Some(hwnd),
            &HSTRING::from(message),
            &HSTRING::from(crate::ui_text::text("action-settings-recovery-title")),
            MB_OK | MB_ICONWARNING,
        );
    }
}

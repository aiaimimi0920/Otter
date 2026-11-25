//! File/cache settings save coordination; preserve configuration extensions, check owner and source before publication, and do not automatically make it the startup path.
use super::*;
use crate::{app_settings_document::Document, file_naming_settings::Settings};
use std::{
    os::windows::ffi::OsStringExt,
    path::PathBuf,
    sync::{atomic::AtomicBool, Arc},
};
use windows::Win32::UI::Controls::Dialogs::*;

pub(super) struct Pending {
    origin: crate::workspace_id::UnitId,
    owner: std::rc::Rc<std::cell::Cell<bool>>,
    before: Settings,
    before_cache: crate::cache_policy::Policy,
    before_recycle: crate::session_library::recycle::policy::Policy,
    source: Arc<Document>,
    job: crate::desktop::naming_save_job::Job,
    automatic: bool,
}
pub(super) unsafe fn request(hwnd: HWND, state: &State) -> Result<()> {
    request_kind(hwnd, state, false)
}
pub(super) unsafe fn request_update(hwnd: HWND, state: &State) -> Result<()> {
    request_kind(hwnd, state, true)
}
unsafe fn request_kind(hwnd: HWND, state: &State, update: bool) -> Result<()> {
    if state.modals.blocked() {
        return Ok(());
    }
    if !IsWindow(Some(hwnd)).as_bool() {
        return Err(Error::new(E_ABORT, "Naming requester closed"));
    }
    let flag = if update {
        &state.naming_update_requested
    } else {
        &state.naming_save_requested
    };
    if !flag.replace(true) {
        if let Err(error) = PostMessageW(None, WM_NULL, WPARAM(0), LPARAM(0)) {
            flag.set(false);
            return Err(error);
        }
    }
    Ok(())
}
impl Application {
    pub(super) fn naming_save_reap_deadline(&self) -> Option<std::time::Instant> {
        if self.modals.blocked() || self.locale_busy() {
            return None;
        }
        self.naming_save_job.as_ref()?.job.deadline()
    }

    pub(super) unsafe fn pending_naming_save(&mut self) {
        let message = match self.pending_naming_save_auto_with(|hwnd, state| choose(hwnd, state)) {
            Ok(warning) => warning,
            Err(error) => Some(crate::ui_text::format(
                "action-naming-save-failed",
                &[("error", error.to_string().into())],
            )),
        };
        if let Some(message) = message {
            MessageBoxW(
                None,
                &HSTRING::from(message),
                w!("Otter"),
                MB_OK | MB_ICONWARNING,
            );
        }
    }
    pub(super) unsafe fn pending_naming_save_with(
        &mut self,
        choose: impl FnOnce(HWND, &State) -> Result<Option<PathBuf>>,
    ) -> Result<Option<String>> {
        if self.modals.blocked() || self.locale_busy() {
            return Ok(None);
        }
        if let Some(pending) = self.naming_save_job.as_mut() {
            let Some(result) = pending.job.poll() else {
                return Ok(None);
            };
            let pending = self.naming_save_job.take().expect("polled job exists");
            // Schedule the next turn before settling; do not swallow explicit requests even if this task fails or an automatic result is discarded.
            if self.documents.iter().any(|doc| {
                doc.is_alive()
                    && (doc.state.naming_save_requested.get()
                        || doc.state.naming_update_requested.get())
            }) {
                self.execution_wake.callback()();
            }
            let mut warning = None;
            if pending.automatic && !self.naming_autosave.get() {
                // Disabling autosave discards unpublished automatic results; manual saves are unaffected.
                return Ok(None);
            }
            let staged = result.map_err(|message| Error::new(E_FAIL, message))?;
            if !self.documents.iter().any(|doc| {
                doc.state.unit_id == pending.origin
                    && doc.is_alive()
                    && std::rc::Rc::ptr_eq(&doc.state.file_dialog, &pending.owner)
            }) || *self.naming.borrow() != pending.before
                || self.cache_policy.get() != pending.before_cache
                || self.recycle_policy != pending.before_recycle
                || !Arc::ptr_eq(&self.naming_document, &pending.source)
            {
                return Err(Error::new(
                    E_ABORT,
                    "Naming settings or source changed during save",
                ));
            }
            match staged {
                crate::desktop::naming_save_job::Ready::New(staged, next) => {
                    staged
                        .commit_new(&AtomicBool::new(false))
                        .map_err(|e| Error::new(E_FAIL, e.to_string()))?;
                    self.naming_document = next.document().clone();
                    self.naming_source = Some(next);
                    self.schedule_recycle_maintenance();
                }
                crate::desktop::naming_save_job::Ready::Update(prepared) => {
                    let (next, published) =
                        prepared
                            .commit(&AtomicBool::new(false))
                            .map_err(|failure| {
                                Error::new(
                                    E_FAIL,
                                    format!(
                                        "{}; recovery={:?}; restore_error={:?}",
                                        failure.error, failure.recovery, failure.restore_error
                                    ),
                                )
                            })?;
                    self.naming_document = next.document().clone();
                    self.naming_source = Some(next);
                    warning = published.retained_recovery.map(|(path, error)| {
                        crate::ui_text::format(
                            "action-naming-save-warning",
                            &[
                                ("path", path.display().to_string().into()),
                                ("error", error.to_string().into()),
                            ],
                        )
                    });
                    self.schedule_recycle_maintenance();
                }
            }
            return Ok(warning);
        }
        let automatic = std::mem::take(&mut self.naming_autosave_request);
        let mut requester = None;
        for doc in &self.documents {
            let update = doc.state.naming_update_requested.replace(false);
            let save_as = doc.state.naming_save_requested.replace(false);
            if (update || save_as)
                && doc.is_alive()
                && requester
                    .as_ref()
                    .is_none_or(|(_, _, existing_update)| save_as && *existing_update)
            {
                // Save-as takes priority within the same batch; do not implicitly turn a new-file selection into an overwrite.
                requester = Some((doc.state.unit_id.clone(), doc.window.0, update && !save_as));
            }
        }
        let Some((origin, hwnd, update)) = requester else {
            return Ok(None);
        };
        let before = self.naming.borrow().clone();
        let before_cache = self.cache_policy.get();
        let before_recycle = self.recycle_policy;
        let source = self.naming_document.clone();
        let doc = self
            .documents
            .iter()
            .find(|doc| doc.state.unit_id == origin)
            .ok_or_else(|| Error::new(E_ABORT, "Naming requester closed"))?;
        let loaded = if update {
            self.naming_source.clone()
        } else {
            None
        };
        if update && loaded.is_none() && self.naming_profile_target.is_none() {
            return Err(Error::new(
                E_INVALIDARG,
                crate::ui_text::text("action-naming-no-source"),
            ));
        }
        if let Some(loaded) = &loaded {
            if loaded.recovery_target().is_some() {
                return Err(Error::new(
                    E_INVALIDARG,
                    crate::ui_text::text("action-naming-recovery-update"),
                ));
            }
            if !Arc::ptr_eq(loaded.document(), &source) {
                return Err(Error::new(
                    E_ABORT,
                    "Settings source no longer matches document",
                ));
            }
        }
        let path = if let Some(loaded) = &loaded {
            loaded.path().to_path_buf()
        } else if update {
            self.naming_profile_target
                .clone()
                .expect("profile target checked")
        } else if let Some(path) = choose(hwnd, &doc.state)? {
            path
        } else {
            return Ok(None);
        };
        let owner = doc.state.file_dialog.clone();
        if self.source_window(&origin) != Some(hwnd)
            || *self.naming.borrow() != before
            || self.cache_policy.get() != before_cache
            || self.recycle_policy != before_recycle
            || !Arc::ptr_eq(&self.naming_document, &source)
        {
            return Err(Error::new(
                E_ABORT,
                "Naming settings changed during selection",
            ));
        }
        if !update
            && path
                .try_exists()
                .map_err(|e| Error::new(E_FAIL, e.to_string()))?
        {
            return Err(Error::new(
                E_INVALIDARG,
                crate::ui_text::text("action-naming-new-file"),
            ));
        }
        let job = if let Some(loaded) = loaded {
            crate::desktop::naming_save_job::Job::start_update(
                loaded,
                before.clone(),
                before_cache,
                before_recycle,
                self.execution_wake.callback(),
            )
        } else {
            crate::desktop::naming_save_job::Job::start(
                path,
                source.clone(),
                before.clone(),
                before_cache,
                before_recycle,
                update,
                self.execution_wake.callback(),
            )
        }
        .map_err(|e| Error::new(E_FAIL, e.to_string()))?;
        self.naming_save_job = Some(Pending {
            origin,
            owner,
            before,
            before_cache,
            before_recycle,
            source,
            job,
            automatic: automatic && update,
        });
        Ok(None)
    }
}
unsafe fn choose(hwnd: HWND, state: &State) -> Result<Option<PathBuf>> {
    let flag = state.file_dialog.clone();
    let _dialog = crate::desktop::file_dialog::Guard::begin(&flag)
        .ok_or_else(|| Error::new(E_ABORT, "Another dialog is active"))?;
    let mut name = vec![0u16; 32768];
    let title = HSTRING::from(crate::ui_text::text("action-naming-save-title"));
    for (target, value) in name.iter_mut().zip("otter-settings.json".encode_utf16()) {
        *target = value;
    }
    let mut options = OPENFILENAMEW {
        lStructSize: std::mem::size_of::<OPENFILENAMEW>() as u32,
        hwndOwner: hwnd,
        lpstrFile: windows::core::PWSTR(name.as_mut_ptr()),
        nMaxFile: name.len() as u32,
        lpstrFilter: w!("Otter settings\0*.json\0\0"),
        lpstrDefExt: w!("json"),
        lpstrTitle: windows::core::PCWSTR(title.as_ptr()),
        Flags: OFN_EXPLORER | OFN_PATHMUSTEXIST | OFN_NOCHANGEDIR,
        ..Default::default()
    };
    if !GetSaveFileNameW(&mut options).as_bool() {
        return if CommDlgExtendedError() == COMMON_DLG_ERRORS(0) {
            Ok(None)
        } else {
            Err(Error::new(E_FAIL, "Naming settings save dialog failed"))
        };
    }
    let end = name
        .iter()
        .position(|value| *value == 0)
        .ok_or_else(|| Error::new(E_INVALIDARG, "Unterminated settings path"))?;
    Ok(Some(PathBuf::from(std::ffi::OsString::from_wide(
        &name[..end],
    ))))
}

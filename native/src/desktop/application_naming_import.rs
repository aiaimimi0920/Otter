//! 显式导入命名偏好；文件不修改，完成时验证原始 owner 与设置快照。
use super::*;
use crate::file_naming_settings::Settings;
use std::{os::windows::ffi::OsStringExt, path::PathBuf};
use windows::Win32::UI::Controls::Dialogs::*;

pub(super) struct Pending {
    origin: crate::workspace_id::UnitId,
    // 同 UnitId 的工作区恢复可能重建窗口；保留轻量 owner 身份，不保留窗口或像素。
    origin_dialog: std::rc::Rc<std::cell::Cell<bool>>,
    before: Settings,
    before_recycle: crate::session_library::recycle::policy::Policy,
    job: crate::desktop::naming_import_job::Job,
}
pub(super) unsafe fn request(hwnd: HWND, state: &State) -> Result<()> {
    if state.modals.blocked() {
        return Ok(());
    }
    if !IsWindow(Some(hwnd)).as_bool() {
        return Err(Error::new(E_ABORT, "Naming requester closed"));
    }
    if !state.naming_import_requested.replace(true) {
        if let Err(error) = PostMessageW(None, WM_NULL, WPARAM(0), LPARAM(0)) {
            state.naming_import_requested.set(false);
            return Err(error);
        }
    }
    Ok(())
}
impl Application {
    pub(in crate::desktop) fn install_profile_target(
        &mut self,
        target: Option<std::path::PathBuf>,
    ) {
        self.naming_profile_target = target;
    }
    pub(in crate::desktop) fn install_startup_settings(
        &mut self,
        source: std::sync::Arc<crate::settings_source::Source>,
    ) -> Result<()> {
        let recycle = source
            .document()
            .recycle_policy()
            .map_err(|error| Error::new(E_INVALIDARG, error))?;
        for doc in &self.documents {
            crate::desktop::settings_startup::verify(&doc.state, source.document())?;
        }
        self.art_issuers
            .replace(
                source
                    .document()
                    .art_issuer_trust()
                    .map_err(|error| Error::new(E_INVALIDARG, error))?,
            )
            .map_err(|error| Error::new(E_INVALIDARG, error))?;
        self.naming_document = source.document().clone();
        self.recycle_policy = recycle;
        self.naming_source = Some(source);
        self.schedule_recycle_maintenance();
        Ok(())
    }

    pub(in crate::desktop) fn verify_startup_naming(&self) -> Result<()> {
        for doc in &self.documents {
            crate::desktop::settings_startup::verify(&doc.state, &self.naming_document)?;
            if !std::rc::Rc::ptr_eq(&doc.state.naming, &self.naming) {
                return Err(Error::new(
                    E_FAIL,
                    "Restored naming owner differs from startup owner",
                ));
            }
            if !std::rc::Rc::ptr_eq(&doc.state.cache_policy, &self.cache_policy) {
                return Err(Error::new(
                    E_FAIL,
                    "Restored cache policy owner differs from startup owner",
                ));
            }
        }
        println!("native-settings-restore: applied=true shared=true source-readonly=true");
        Ok(())
    }
    pub(super) unsafe fn pending_naming_import(&mut self) {
        if let Err(error) = self.pending_naming_import_confirm_with(
            |hwnd, state| choose(hwnd, state),
            |hwnd, before, next| super::art_issuer_confirmation::confirm(hwnd, before, next),
        ) {
            MessageBoxW(
                None,
                &HSTRING::from(format!("命名设置未导入。\n{error}")),
                w!("Otter"),
                MB_OK | MB_ICONWARNING,
            );
        }
    }
    #[cfg(test)]
    unsafe fn pending_naming_import_with(
        &mut self,
        choose: impl FnOnce(HWND, &State) -> Result<Option<PathBuf>>,
    ) -> Result<()> {
        self.pending_naming_import_confirm_with(choose, |_, _, _| Ok(false))
    }
    unsafe fn pending_naming_import_confirm_with(
        &mut self,
        choose: impl FnOnce(HWND, &State) -> Result<Option<PathBuf>>,
        confirm: impl FnOnce(
            HWND,
            &crate::art_issuer_trust::Document,
            &crate::art_issuer_trust::Document,
        ) -> Result<bool>,
    ) -> Result<()> {
        if self.modals.blocked() {
            return Ok(());
        }
        // 新操作先消费，不能将忙碌时的点击变为完成后第二次隐式导入。
        let mut requester = None;
        for doc in &self.documents {
            if doc.state.naming_import_requested.replace(false)
                && doc.is_alive()
                && requester.is_none()
            {
                requester = Some((doc.state.unit_id.clone(), doc.window.0));
            }
        }
        let was_busy = self.naming_import_job.is_some();
        if let Some(result) = self
            .naming_import_job
            .as_mut()
            .and_then(|pending| pending.job.poll())
        {
            let pending = self.naming_import_job.take().expect("polled job exists");
            let next = result.map_err(|message| Error::new(E_FAIL, message))?;
            let recycle = next
                .document()
                .recycle_policy()
                .map_err(|error| Error::new(E_INVALIDARG, error))?;
            if self.recycle_policy != pending.before_recycle {
                return Err(Error::new(
                    E_ABORT,
                    "Recycle settings changed during import",
                ));
            }
            if !self.documents.iter().any(|doc| {
                doc.state.unit_id == pending.origin
                    && doc.is_alive()
                    && std::rc::Rc::ptr_eq(&doc.state.file_dialog, &pending.origin_dialog)
            }) {
                return Err(Error::new(E_ABORT, "Naming import requester closed"));
            }
            let before_document = self.naming_document.clone();
            let before_trust = before_document
                .art_issuer_trust()
                .map_err(|e| Error::new(E_INVALIDARG, e))?;
            let next_trust = next
                .document()
                .art_issuer_trust()
                .map_err(|e| Error::new(E_INVALIDARG, e))?;
            if before_trust != next_trust {
                let hwnd = self
                    .source_window(&pending.origin)
                    .ok_or_else(|| Error::new(E_ABORT, "Issuer requester closed"))?;
                let confirmed = {
                    let _guard = crate::desktop::file_dialog::Guard::begin(&pending.origin_dialog)
                        .ok_or_else(|| Error::new(E_ABORT, "Another dialog is active"))?;
                    confirm(hwnd, &before_trust, &next_trust)?
                };
                if !confirmed {
                    return Err(Error::new(
                        E_ABORT,
                        "Art issuer trust change was not confirmed",
                    ));
                }
                if !std::sync::Arc::ptr_eq(&before_document, &self.naming_document)
                    || self.source_window(&pending.origin) != Some(hwnd)
                    || self.recycle_policy != pending.before_recycle
                    || !self.documents.iter().any(|doc| {
                        doc.state.unit_id == pending.origin
                            && doc.is_alive()
                            && std::rc::Rc::ptr_eq(&doc.state.file_dialog, &pending.origin_dialog)
                    })
                {
                    return Err(Error::new(
                        E_ABORT,
                        "Settings owner changed during issuer confirmation",
                    ));
                }
            }
            install(
                &mut self.naming.borrow_mut(),
                &pending.before,
                next.document().naming().clone(),
            )?;
            self.art_issuers
                .replace(next_trust)
                .map_err(|e| Error::new(E_INVALIDARG, e))?;
            self.recycle_policy = recycle;
            self.naming_document = next.document().clone();
            self.naming_source = Some(next);
            self.schedule_recycle_maintenance();
        }
        let Some((origin, hwnd)) = requester else {
            return Ok(());
        };
        if was_busy {
            return Ok(());
        }
        let before = self.naming.borrow().clone();
        let before_recycle = self.recycle_policy;
        let doc = self
            .documents
            .iter()
            .find(|doc| doc.state.unit_id == origin)
            .ok_or_else(|| Error::new(E_ABORT, "Naming requester closed"))?;
        let Some(path) = choose(hwnd, &doc.state)? else {
            return Ok(());
        };
        let origin_dialog = doc.state.file_dialog.clone();
        if self.source_window(&origin) != Some(hwnd)
            || *self.naming.borrow() != before
            || self.recycle_policy != before_recycle
        {
            return Err(Error::new(
                E_ABORT,
                "Naming settings changed during selection",
            ));
        }
        let job =
            crate::desktop::naming_import_job::Job::start(path, self.execution_wake.callback())
                .map_err(|e| Error::new(E_FAIL, e.to_string()))?;
        self.naming_import_job = Some(Pending {
            origin,
            origin_dialog,
            before,
            before_recycle,
            job,
        });
        Ok(())
    }
}
fn install(current: &mut Settings, before: &Settings, next: Settings) -> Result<()> {
    if current != before {
        return Err(Error::new(E_ABORT, "Naming settings changed during import"));
    }
    next.validate()
        .map_err(|e| Error::new(E_INVALIDARG, format!("Invalid naming settings: {e:?}")))?;
    *current = next;
    Ok(())
}
unsafe fn choose(hwnd: HWND, state: &State) -> Result<Option<PathBuf>> {
    let flag = state.file_dialog.clone();
    let _dialog = crate::desktop::file_dialog::Guard::begin(&flag)
        .ok_or_else(|| Error::new(E_ABORT, "Another dialog is active"))?;
    let mut name = vec![0u16; 32768];
    let mut options = OPENFILENAMEW {
        lStructSize: std::mem::size_of::<OPENFILENAMEW>() as u32,
        hwndOwner: hwnd,
        lpstrFile: windows::core::PWSTR(name.as_mut_ptr()),
        nMaxFile: name.len() as u32,
        lpstrFilter: w!("Otter settings\0*.json\0\0"),
        lpstrTitle: w!("导入命名设置（仅本次运行，不修改源文件）"),
        Flags: OFN_EXPLORER | OFN_FILEMUSTEXIST | OFN_PATHMUSTEXIST | OFN_NOCHANGEDIR,
        ..Default::default()
    };
    if !GetOpenFileNameW(&mut options).as_bool() {
        return if CommDlgExtendedError() == COMMON_DLG_ERRORS(0) {
            Ok(None)
        } else {
            Err(Error::new(E_FAIL, "Naming settings dialog failed"))
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

#[cfg(test)]
#[path = "application_naming_import_tests.rs"]
mod tests;

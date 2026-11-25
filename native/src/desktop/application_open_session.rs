//! Explicit workspace opening: callbacks enqueue and the application loop selects files; session asset directories are not automatically authorized.
use super::*;
use std::{os::windows::ffi::OsStringExt, path::PathBuf};
use windows::Win32::UI::Controls::Dialogs::*;
#[cfg(any(test, debug_assertions, feature = "smoke-test"))]
#[path = "application_open_session_smoke.rs"]
pub(super) mod smoke;

pub(super) struct Selection {
    pub path: PathBuf,
    pub save_first: bool,
    pub root: Option<crate::desktop::session_asset_root::Root>,
}
pub(super) struct Target {
    pub path: PathBuf,
    pub root: Option<crate::desktop::session_asset_root::Root>,
}
impl From<PathBuf> for Selection {
    fn from(path: PathBuf) -> Self {
        Self {
            path,
            save_first: false,
            root: None,
        }
    }
}

pub(in crate::desktop) unsafe fn request(hwnd: HWND, state: &State) -> Result<()> {
    request_mode(hwnd, state, false)
}
pub(in crate::desktop) unsafe fn request_assets(hwnd: HWND, state: &State) -> Result<()> {
    request_mode(hwnd, state, true)
}
unsafe fn request_mode(hwnd: HWND, state: &State, assets: bool) -> Result<()> {
    if state.modals.blocked() {
        return Ok(());
    }
    if !IsWindow(Some(hwnd)).as_bool() {
        return Err(Error::new(E_INVALIDARG, "Open workspace requester closed"));
    }
    if !state.open_session_requested.replace(true) {
        if let Err(error) = PostMessageW(None, WM_NULL, WPARAM(0), LPARAM(0)) {
            state.open_session_requested.set(false);
            return Err(error);
        }
    }
    if assets {
        state.open_session_assets_requested.set(true);
    }
    Ok(())
}

impl Application {
    pub(super) unsafe fn pending_open_session(&mut self) {
        if let Err(error) =
            self.pending_open_session_with(|app, hwnd, assets| app.choose_session(hwnd, assets))
        {
            MessageBoxW(
                self.documents.first().map(|doc| doc.window.0),
                &HSTRING::from(crate::ui_text::format(
                    "app-workspace-open-failed",
                    &[("error", error.to_string().into())],
                )),
                w!("Otter"),
                MB_OK | MB_ICONWARNING,
            );
        }
    }

    pub(super) unsafe fn pending_open_session_with(
        &mut self,
        choose: impl FnOnce(&Self, HWND, bool) -> Result<Option<Selection>>,
    ) -> Result<()> {
        if self.modals.blocked()
            || self.workspace_autosave_pending.is_some()
            || self.checkpoint_pending()
            || self.checkpoint_exit_waiting()
        {
            return Ok(());
        }
        let mut source = None;
        for doc in &self.documents {
            let assets = doc.state.open_session_assets_requested.replace(false);
            if doc.state.open_session_requested.replace(false) && source.is_none() {
                // Coalesce to one chooser without borrowing consent from a different HWND.
                source = Some((doc.window.0, assets));
            }
        }
        let Some((hwnd, assets)) = source else {
            return Ok(());
        };
        if self.save_job.is_some() || self.restore_job.is_some() || self.exit_after_save {
            self.open_after_save = None;
            return Err(Error::new(
                E_ABORT,
                crate::ui_text::text("app-workspace-open-busy"),
            ));
        }
        let guard = restore_batch::guard::Guard::capture(self)?;
        if !guard.matches(self) {
            return Err(Error::new(
                E_ABORT,
                "Workspace is busy before file selection",
            ));
        }
        let Some(selected) = choose(self, hwnd, assets)? else {
            return Ok(());
        };
        if !IsWindow(Some(hwnd)).as_bool() || !guard.matches(self) {
            return Err(Error::new(
                E_ABORT,
                "Workspace changed while choosing session",
            ));
        }
        // Selecting JSON authorizes reading only that file; external image roots require separate explicit asset admission.
        if selected.save_first {
            let doc = self
                .documents
                .iter()
                .find(|doc| doc.window.0 == hwnd)
                .ok_or_else(|| Error::new(E_ABORT, "Open workspace requester closed"))?;
            save_ui::request_existing(hwnd, &doc.state)?;
            self.open_after_save = Some(Target {
                path: selected.path,
                root: selected.root,
            });
            Ok(())
        } else {
            self.open_after_save = None;
            self.request_restore_input(
                super::super::session_job::Input::File(selected.path),
                selected.root,
                None,
            )
        }
    }

    /// Consume once, only after this save succeeds; retain no old snapshot and never let a future manual save revive canceled open intent.
    pub(super) unsafe fn finish_open_save(&mut self, success: bool) -> Result<bool> {
        let Some(target) = self.open_after_save.take() else {
            return Ok(false);
        };
        if !success
            || self.save_status() != save_state::Status::Unchanged
            || self.exit_requested.get()
            || self.exit_after_save
            || self.documents.iter().any(|doc| {
                doc.state.open_session_requested.get()
                    || doc.state.save_session_requested.get()
                    || doc.state.save_existing_requested.get()
            })
        {
            return Ok(false);
        }
        self.request_restore_input(
            super::super::session_job::Input::File(target.path),
            target.root,
            None,
        )?;
        Ok(true)
    }

    unsafe fn choose_session(&self, hwnd: HWND, assets: bool) -> Result<Option<Selection>> {
        let doc = self
            .documents
            .iter()
            .find(|doc| doc.window.0 == hwnd)
            .ok_or_else(|| Error::new(E_ABORT, "Open workspace requester closed"))?;
        let flag = doc.state.file_dialog.clone();
        let _dialog = super::super::file_dialog::Guard::begin(&flag)
            .ok_or_else(|| Error::new(E_ABORT, "Another file dialog is active"))?;
        let mut name = vec![0u16; 32768];
        let title = HSTRING::from(crate::ui_text::text("app-workspace-open-title"));
        let mut options = OPENFILENAMEW {
            lStructSize: std::mem::size_of::<OPENFILENAMEW>() as u32,
            hwndOwner: hwnd,
            lpstrFile: windows::core::PWSTR(name.as_mut_ptr()),
            nMaxFile: name.len() as u32,
            lpstrFilter: w!("Otter workspace\0*.json\0\0"),
            lpstrTitle: windows::core::PCWSTR(title.as_ptr()),
            Flags: OFN_EXPLORER | OFN_FILEMUSTEXIST | OFN_PATHMUSTEXIST | OFN_NOCHANGEDIR,
            ..Default::default()
        };
        if !GetOpenFileNameW(&mut options).as_bool() {
            let error = CommDlgExtendedError();
            return if error == COMMON_DLG_ERRORS(0) {
                Ok(None)
            } else {
                Err(Error::new(
                    E_FAIL,
                    format!("Open workspace dialog failed: {error:?}"),
                ))
            };
        }
        let root = if assets {
            let Some(root) = crate::desktop::session_asset_dialog::choose(hwnd)? else {
                return Ok(None);
            };
            Some(root)
        } else {
            None
        };
        let save_first = if self.save_status() != save_state::Status::Unchanged {
            match MessageBoxW(
                Some(hwnd),
                &windows::core::HSTRING::from(crate::ui_text::text(
                    "app-workspace-open-confirmation",
                )),
                &windows::core::HSTRING::from(crate::ui_text::text("app-workspace-open-title")),
                MB_YESNOCANCEL | MB_ICONWARNING | MB_DEFBUTTON3,
            ) {
                IDYES => true,
                IDNO => false,
                _ => return Ok(None),
            }
        } else {
            false
        };
        let end = name
            .iter()
            .position(|value| *value == 0)
            .ok_or_else(|| Error::new(E_INVALIDARG, "Unterminated workspace path"))?;
        Ok(Some(Selection {
            path: PathBuf::from(std::ffi::OsString::from_wide(&name[..end])),
            save_first,
            root,
        }))
    }
}

#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-save")))]
mod tests {
    use super::*;

    #[test]
    fn open_session_coalescing_keeps_assets_mode_with_its_requester() {
        unsafe {
            let _apartment = crate::desktop::Apartment::new().unwrap();
            let instance = crate::desktop::workspace_window_fixture::register();
            let first =
                DocumentWindow::create(instance, State::new(None, false).unwrap(), false).unwrap();
            let mut app = Application::new(instance, first);
            let second =
                DocumentWindow::create(instance, State::new(None, false).unwrap(), false).unwrap();
            app.workspace
                .add_units([second.state.unit_id.clone()])
                .unwrap();
            app.documents.push(second);
            let one = &app.documents[0];
            let two = &app.documents[1];
            let first_hwnd = one.window.0;
            request(first_hwnd, &one.state).unwrap();
            request_assets(two.window.0, &two.state).unwrap();
            let mut calls = 0;
            app.pending_open_session_with(|_, hwnd, assets| {
                calls += 1;
                assert_eq!(hwnd, first_hwnd);
                assert!(!assets);
                Ok(None)
            })
            .unwrap();
            assert_eq!(calls, 1);
            app.pending_open_session_with(|_, _, _| panic!("coalesced request replayed"))
                .unwrap();

            let one = &app.documents[0];
            request(first_hwnd, &one.state).unwrap();
            request_assets(first_hwnd, &one.state).unwrap();
            app.pending_open_session_with(|_, hwnd, assets| {
                calls += 1;
                assert_eq!(hwnd, first_hwnd);
                assert!(assets);
                Ok(None)
            })
            .unwrap();
            assert_eq!(calls, 2);
            assert!(app.restore_job.is_none());
        }
    }
}

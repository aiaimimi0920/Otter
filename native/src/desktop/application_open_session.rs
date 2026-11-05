//! 显式打开工作区：回调排队，应用循环选文件；不自动授权会话资产目录。
use super::*;
use std::{os::windows::ffi::OsStringExt, path::PathBuf};
use windows::Win32::UI::Controls::Dialogs::*;
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
                &HSTRING::from(format!("无法打开工作区，原工作区保留。\n{error}")),
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
        let mut assets = false;
        for doc in &self.documents {
            assets |= doc.state.open_session_assets_requested.replace(false);
            if doc.state.open_session_requested.replace(false) && source.is_none() {
                source = Some(doc.window.0);
            }
        }
        let Some(hwnd) = source else {
            return Ok(());
        };
        if self.save_job.is_some() || self.restore_job.is_some() || self.exit_after_save {
            self.open_after_save = None;
            return Err(Error::new(
                E_ABORT,
                "工作区正在保存、恢复或等待退出，请完成后重试。",
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
        // 选择 JSON 只授权读取该文件；外部图片根必须经独立的显式资产准入。
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

    /// 只在本次保存成功后消费一次；不保留旧快照，不让未来手动保存复活取消的打开意图。
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
        let mut options = OPENFILENAMEW {
            lStructSize: std::mem::size_of::<OPENFILENAMEW>() as u32,
            hwndOwner: hwnd,
            lpstrFile: windows::core::PWSTR(name.as_mut_ptr()),
            nMaxFile: name.len() as u32,
            lpstrFilter: w!("Otter workspace\0*.json\0\0"),
            lpstrTitle: w!("打开工作区"),
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
            match MessageBoxW(Some(hwnd),
                w!("打开前保存当前工作区吗？\n是：保存后打开\n否：放弃未保存内容并打开\n取消：继续编辑"),
                w!("打开工作区"), MB_YESNOCANCEL | MB_ICONWARNING | MB_DEFBUTTON3) {
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

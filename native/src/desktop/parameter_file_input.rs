//! 文件参数只排队路径和配置凭证；系统文件框中不借用参数，不执行读取。
use super::{configuration_stamp::Stamp, State};
use std::{path::PathBuf, sync::Arc};
use windows::{
    core::{w, Result, PWSTR},
    Win32::{
        Foundation::*,
        UI::{Controls::Dialogs::*, WindowsAndMessaging::*},
    },
};

#[derive(Clone)]
pub(super) enum Target {
    Parameter(String),
    Sticker,
}
pub(super) struct Request {
    pub path: PathBuf,
    pub target: Target,
    pub stamp: Stamp,
    pub view: Option<Arc<()>>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Outcome {
    /// 请求已接纳；文件内容仍由原worker读取，不表示正式参数已提交。
    Accepted,
    Cancelled,
    Rejected,
}
impl Request {
    pub fn matches(&self, state: &State) -> bool {
        if state.group_locked.get()
            || state.group_hidden.get()
            || state.session_end.paused()
            || !super::capture_input::exclusive_available(state)
            || !self.stamp.matches(state)
        {
            return false;
        }
        let Target::Parameter(id) = &self.target else {
            return !state.is_art();
        };
        let selection = state.selected_art.borrow();
        let allowed = selection.as_ref().is_some_and(|selected| {
            selected
                .catalog
                .handshake
                .arts()
                .iter()
                .filter_map(crate::loom_capability::Capability::new)
                .find(|cap| cap.id == selected.parameters.art_id())
                .is_some_and(|cap| {
                    cap.enabled
                        && cap.parameters().any(|p| {
                            p.id == id
                                && !p.secret
                                && p.disabled != Some(true)
                                && matches!(p.widget.as_str(), "file" | "image_link")
                        })
                })
                && selected
                    .parameters
                    .get(id)
                    .flatten()
                    .and_then(serde_json::Value::as_str)
                    != Some(crate::graph_params::DISABLED)
        });
        allowed
            && !state
                .parameter_view
                .borrow()
                .as_ref()
                .is_some_and(|view| view.linked.contains(id))
            && !state.group_locked.get()
            && self.stamp.matches(state)
            && super::parameter_view::matches(&self.view, state)
    }
}
pub(super) unsafe fn choose(
    hwnd: HWND,
    state: &State,
    id: &str,
    stamp: Stamp,
    view: Option<Arc<()>>,
    owner_current: &dyn Fn() -> bool,
) -> Result<bool> {
    Ok(choose_target(
        hwnd,
        state,
        Target::Parameter(id.into()),
        stamp,
        view,
        owner_current,
    )? == Outcome::Accepted)
}
pub(super) unsafe fn choose_target(
    hwnd: HWND,
    state: &State,
    target: Target,
    stamp: Stamp,
    view: Option<Arc<()>>,
    owner_current: &dyn Fn() -> bool,
) -> Result<Outcome> {
    if !owner_current() || state.parameter_file_request.borrow().is_some() {
        return Ok(Outcome::Rejected);
    }
    let path = {
        let Some(_guard) = super::file_dialog::Guard::begin(&state.file_dialog) else {
            return Ok(Outcome::Rejected);
        };
        choose_path(hwnd, &target)?
    };
    finish_choice(
        hwnd,
        state,
        path.map(|path| Request {
            path,
            target,
            stamp,
            view,
        }),
        owner_current,
    )
}
unsafe fn choose_path(hwnd: HWND, target: &Target) -> Result<Option<PathBuf>> {
    let path = {
        let mut name = vec![0u16; 32768];
        let mut options = OPENFILENAMEW {
            lStructSize: std::mem::size_of::<OPENFILENAMEW>() as u32,
            hwndOwner: hwnd,
            lpstrFile: PWSTR(name.as_mut_ptr()),
            nMaxFile: name.len() as u32,
            lpstrFilter: if matches!(target, Target::Sticker) {
                w!("图片\0*.png;*.jpg;*.jpeg;*.webp;*.bmp\0\0")
            } else if matches!(target, Target::Parameter(id) if id.contains("image")) {
                w!("图片\0*.png;*.jpg;*.jpeg;*.bmp;*.gif;*.webp;*.tif;*.tiff;*.svg;*.avif\0所有文件\0*.*\0\0")
            } else {
                w!("所有文件\0*.*\0\0")
            },
            Flags: OFN_EXPLORER | OFN_FILEMUSTEXIST | OFN_PATHMUSTEXIST | OFN_NOCHANGEDIR,
            ..Default::default()
        };
        if !GetOpenFileNameW(&mut options).as_bool() {
            let error = CommDlgExtendedError();
            if error != COMMON_DLG_ERRORS(0) {
                return Err(windows::core::Error::new(
                    E_FAIL,
                    format!("Open dialog failed: {error:?}"),
                ));
            }
            return Ok(None);
        }
        use std::os::windows::ffi::OsStringExt;
        let end = name
            .iter()
            .position(|value| *value == 0)
            .unwrap_or(name.len());
        PathBuf::from(std::ffi::OsString::from_wide(&name[..end]))
    };
    Ok(Some(path))
}
unsafe fn finish_choice(
    hwnd: HWND,
    state: &State,
    request: Option<Request>,
    owner_current: &dyn Fn() -> bool,
) -> Result<Outcome> {
    // 取消不是准入失败，且不得伪造请求、配置变化或提交凭证。
    let Some(request) = request else {
        return Ok(Outcome::Cancelled);
    };
    if !owner_current() {
        return Ok(Outcome::Rejected);
    }
    Ok(if queue(hwnd, state, request)? {
        Outcome::Accepted
    } else {
        Outcome::Rejected
    })
}
pub(super) unsafe fn queue(hwnd: HWND, state: &State, request: Request) -> Result<bool> {
    if !IsWindow(Some(hwnd)).as_bool()
        || !request.matches(state)
        || state.file_dialog.get()
        || state.parameter_file_request.borrow().is_some()
    {
        return Ok(false);
    }
    *state.parameter_file_request.borrow_mut() = Some(request);
    if let Err(error) = PostMessageW(Some(hwnd), WM_NULL, WPARAM(0), LPARAM(0)) {
        state.parameter_file_request.borrow_mut().take();
        return Err(error);
    }
    Ok(true)
}

#[cfg(test)]
#[path = "parameter_file_input_tests.rs"]
mod tests;

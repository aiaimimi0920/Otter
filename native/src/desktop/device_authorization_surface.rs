//! Art 选择只排队，不在模态回调执行网络；保留发起窗口的稳定节点身份。
use super::*;
pub(super) unsafe fn choose(
    hwnd: HWND,
    state: &State,
    choose: impl FnOnce() -> Option<String>,
) -> Result<()> {
    if state.modals.blocked() || state.exit_requested.get() {
        return Ok(());
    }
    if !IsWindow(Some(hwnd)).as_bool() {
        return Err(Error::new(E_ABORT, "Surface requester closed"));
    }
    let old = state.authorization.borrow().origin.clone();
    if old.is_none() {
        return Err(Error::new(E_ABORT, "Select a Loom service first"));
    }
    let flag = state.file_dialog.clone();
    let guard = super::super::file_dialog::Guard::begin(&flag)
        .ok_or_else(|| Error::new(E_ABORT, "Another dialog is active"))?;
    let art = choose();
    drop(guard);
    let Some(art) = art else {
        return Ok(());
    };
    if state.authorization.borrow().origin != old {
        return Err(Error::new(E_ABORT, "Service changed while selecting Art"));
    }
    if !crate::live_relay_viewer_identity::identifier(&art) {
        return Err(Error::new(E_INVALIDARG, "Invalid Art ID"));
    }
    queue(
        hwnd,
        state,
        Request::OpenArt {
            art,
            node: state.unit_id.as_str().into(),
        },
    )
}

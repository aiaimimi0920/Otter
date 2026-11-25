//! Favorite commands enqueue only the current window and source credential; background encoding/publication never enters window callbacks.
use super::*;
pub(super) const SET: usize = 0x10b0;
pub(super) const REMOVE: usize = 0x10b1;
pub(super) const RECYCLE: usize = 0x10b2;
pub(super) const CONFIGURE: usize = 0x10b3;
pub(super) const CLEAR_RECYCLE: usize = 0x10b4;
pub(super) const CLEAR_REFERENCE: usize = 0x10b5;
pub(super) const RECYCLE_ROOT: usize = 0x10b6;
pub(super) const REFERENCE_ROOT: usize = 0x10b7;
#[derive(Clone, Copy, PartialEq)]
pub(super) enum Action {
    Set,
    Remove,
    Recycle,
    ClearRecycle,
    ClearReference,
}
pub(super) struct Request {
    pub source: crate::session_source::Source,
    pub unit: crate::workspace_id::UnitId,
    pub target: popup_menu::target::MenuTarget,
    pub action: Action,
}
pub(super) fn handles(command: usize) -> bool {
    matches!(
        command,
        SET | REMOVE | RECYCLE | CONFIGURE | CLEAR_RECYCLE | CLEAR_REFERENCE
    )
}
pub(super) unsafe fn request(hwnd: HWND, state: &State, command: usize) -> Result<()> {
    if !handles(command) {
        return Err(Error::new(E_INVALIDARG, "Unknown reference command"));
    }
    if state.modals.blocked()
        || (state.is_art() && matches!(command, SET | REMOVE | RECYCLE))
        || !IsWindow(Some(hwnd)).as_bool()
    {
        return Ok(());
    }
    let mut view = state.frozen_library.borrow_mut();
    if view.busy
        || view.request.is_some()
        || view.reference.is_some()
        || view.configure_recycle.is_some()
    {
        return Ok(());
    }
    if command == CONFIGURE {
        view.configure_recycle = Some(state.unit_id.clone());
        if let Err(error) = PostMessageW(None, WM_NULL, WPARAM(0), LPARAM(0)) {
            view.configure_recycle = None;
            return Err(error);
        }
        return Ok(());
    }
    let Some(source) = view.source.clone() else {
        return Ok(());
    };
    view.reference = Some(Request {
        source,
        unit: state.unit_id.clone(),
        target: popup_menu::target::MenuTarget::capture(&state.canvas.borrow()),
        action: match command {
            RECYCLE => Action::Recycle,
            REMOVE => Action::Remove,
            CLEAR_RECYCLE => Action::ClearRecycle,
            CLEAR_REFERENCE => Action::ClearReference,
            _ => Action::Set,
        },
    });
    if let Err(error) = PostMessageW(None, WM_NULL, WPARAM(0), LPARAM(0)) {
        view.reference = None;
        return Err(error);
    }
    Ok(())
}
pub(super) unsafe fn append_menu(menu: HMENU, state: &State) -> Result<()> {
    let view = state.frozen_library.borrow();
    let available = !view.busy && view.source.is_some();
    let flags = MF_STRING | if available { MF_ENABLED } else { MF_GRAYED };
    AppendMenuW(
        menu,
        flags
            | if state.is_art() {
                MF_GRAYED
            } else {
                MF_ENABLED
            },
        RECYCLE,
        w!("关闭"),
    )?;
    // The frozen library's original worker replaces placeholder submenus and asynchronously determines favorite/unfavorite state.
    AppendMenuW(menu, flags, RECYCLE_ROOT, w!("回收站"))?;
    AppendMenuW(menu, flags, CLEAR_RECYCLE, w!("清空回收站"))?;
    AppendMenuW(menu, MF_SEPARATOR, 0, None)?;
    AppendMenuW(menu, MF_STRING | MF_GRAYED, SET, w!("收藏"))?;
    AppendMenuW(menu, flags, REFERENCE_ROOT, w!("收藏夹"))?;
    AppendMenuW(menu, flags, CLEAR_REFERENCE, w!("清空收藏夹"))
}

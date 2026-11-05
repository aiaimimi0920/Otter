//! 收藏命令只排队当前窗口和来源凭证；后台编码/发布不进入窗口回调。
use super::*;
pub(super) const SET: usize = 0x10b0;
pub(super) const REMOVE: usize = 0x10b1;
pub(super) const RECYCLE: usize = 0x10b2;
pub(super) const CONFIGURE: usize = 0x10b3;
#[derive(Clone, Copy, PartialEq)]
pub(super) enum Action {
    Set,
    Remove,
    Recycle,
}
pub(super) struct Request {
    pub source: crate::session_source::Source,
    pub unit: crate::workspace_id::UnitId,
    pub target: popup_menu::target::MenuTarget,
    pub action: Action,
}
pub(super) fn handles(command: usize) -> bool {
    matches!(command, SET | REMOVE | RECYCLE | CONFIGURE)
}
pub(super) unsafe fn request(hwnd: HWND, state: &State, command: usize) -> Result<()> {
    if !handles(command) {
        return Err(Error::new(E_INVALIDARG, "Unknown reference command"));
    }
    if state.modals.blocked()
        || (state.is_art() && command != CONFIGURE)
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
    let available = !state.is_art() && !view.busy && view.source.is_some();
    let flags = MF_STRING | if available { MF_ENABLED } else { MF_GRAYED };
    AppendMenuW(menu, flags, SET, w!("收藏当前贴纸（更新快照）"))?;
    AppendMenuW(menu, flags, REMOVE, w!("取消当前贴纸收藏"))?;
    AppendMenuW(menu, flags, RECYCLE, w!("关闭贴纸并移入回收站"))?;
    AppendMenuW(
        menu,
        MF_STRING | if view.busy { MF_GRAYED } else { MF_ENABLED },
        CONFIGURE,
        w!("回收站保留设置…"),
    )
}

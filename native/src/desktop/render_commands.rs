//! 当前文档的手动后端选择/重试；不重置关联门禁、不保存全局偏好、不改变文档和编辑历史。
use super::{canvas::RenderChoice, State};
use windows::{
    core::{w, Error, Result},
    Win32::{Foundation::*, Graphics::Gdi::InvalidateRect, UI::WindowsAndMessaging::*},
};

pub(super) const SOFTWARE: usize = 0x10F0;
pub(super) const RETRY: usize = 0x10F1;
pub(super) const AUTOMATIC: usize = 0x10F2;
pub(super) const COMPATIBILITY: usize = 0x10F3;
pub(super) fn handles(command: usize) -> bool {
    matches!(command, SOFTWARE | RETRY | AUTOMATIC | COMPATIBILITY)
}
fn available(state: &State, retry: bool) -> bool {
    if state.render_active.get() {
        return false;
    }
    state
        .canvas
        .try_borrow()
        .is_ok_and(|canvas| canvas.can_request_render() && (!retry || canvas.software_selected()))
}
pub(super) unsafe fn append(menu: HMENU, state: &State) -> Result<()> {
    let selected = state
        .canvas
        .try_borrow()
        .map_err(|_| Error::new(E_ABORT, "Canvas is in use while creating the render menu"))?
        .render_choice();
    for (command, label, choice) in [
        (
            AUTOMATIC,
            w!("自动选择渲染后端"),
            Some(RenderChoice::Automatic),
        ),
        (
            COMPATIBILITY,
            w!("使用兼容渲染"),
            Some(RenderChoice::Compatibility),
        ),
        (SOFTWARE, w!("使用 CPU 渲染"), Some(RenderChoice::Software)),
        (RETRY, w!("重试 CPU 绘制"), None),
    ] {
        AppendMenuW(
            menu,
            MF_STRING
                | if available(state, command == RETRY) {
                    MF_ENABLED
                } else {
                    MF_GRAYED
                }
                | if choice == Some(selected) {
                    MF_CHECKED
                } else {
                    MF_UNCHECKED
                },
            command,
            label,
        )?;
    }
    Ok(())
}
pub(super) unsafe fn execute(hwnd: HWND, state: &State, command: usize) -> Result<()> {
    // 菜单快照可能已过期；进入事务前再次检查宿主、paint 和退休状态。
    if !handles(command) || !state.lifetime.owns(hwnd) || !available(state, command == RETRY) {
        return Err(Error::new(
            E_ABORT,
            "Manual render selection is not currently available",
        ));
    }
    let choice = match command {
        AUTOMATIC => RenderChoice::Automatic,
        COMPATIBILITY => RenderChoice::Compatibility,
        _ => RenderChoice::Software,
    };
    let pending = state
        .canvas
        .try_borrow_mut()
        .map_err(|_| Error::new(E_ABORT, "Canvas is in use during a manual render request"))?
        .request_render(choice)?;
    state.render_retries.set(0);
    if pending {
        super::procedure::render_recovery::start(hwnd, state);
    } else {
        let _ = InvalidateRect(Some(hwnd), None, false);
    }
    Ok(())
}

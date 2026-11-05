//! 窗口共享的只读库来源与单槽请求；不在窗口回调或菜单消息泵里准备图片。
use super::*;
use crate::{session_library::Kind, session_source::Source, workspace_id::UnitId};
#[derive(Default)]
pub(super) struct View {
    pub source: Option<Source>,
    pub busy: bool,
    pub request: Option<Request>,
    pub reference: Option<super::reference_ui::Request>,
    pub configure_recycle: Option<UnitId>,
}
pub(super) struct Choice {
    pub remove: bool,
    pub source: Source,
    pub kind: Kind,
    pub entry_id: String,
}
pub(super) struct Request {
    pub choice: Choice,
    pub origin: UnitId,
    pub target: popup_menu::target::MenuTarget,
    pub point: POINT,
}
pub(super) unsafe fn enqueue(
    hwnd: HWND,
    state: &State,
    choice: Choice,
    point: POINT,
) -> Result<()> {
    if !IsWindow(Some(hwnd)).as_bool() || state.file_dialog.get() {
        return Ok(());
    }
    let mut view = state.frozen_library.borrow_mut();
    if view.busy
        || view.request.is_some()
        || view.reference.is_some()
        || view.configure_recycle.is_some()
        || view
            .source
            .as_ref()
            .is_none_or(|source| !source.same_storage(&choice.source))
    {
        return Ok(());
    }
    view.request = Some(Request {
        choice,
        origin: state.unit_id.clone(),
        target: popup_menu::target::MenuTarget::capture(&state.canvas.borrow()),
        point,
    });
    // PostMessage 不派发消息；失败撤回本次单槽请求，不留下延迟执行。
    if let Err(error) = PostMessageW(Some(hwnd), WM_NULL, WPARAM(0), LPARAM(0)) {
        view.request = None;
        return Err(error);
    }
    Ok(())
}

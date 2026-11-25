//! Window-shared read-only library provenance and single-slot requests; no image preparation in window callbacks/menu pumps.
use super::*;
use crate::{session_library::Kind, session_source::Source, workspace_id::UnitId};
#[derive(Default)]
pub(super) struct View {
    pub menu: Option<super::context_menu::Pending>,
    pub source: Option<Source>,
    pub busy: bool,
    pub request: Option<Request>,
    pub reference: Option<super::reference_ui::Request>,
    pub configure_recycle: Option<UnitId>,
    pub workers: super::popup_menu::Workers,
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
    // PostMessage dispatches no messages; failure retracts this single-slot request without leaving deferred execution.
    if let Err(error) = PostMessageW(Some(hwnd), WM_NULL, WPARAM(0), LPARAM(0)) {
        view.request = None;
        return Err(error);
    }
    Ok(())
}

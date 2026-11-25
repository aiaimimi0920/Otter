//! All work windows share authorization requests/display state; no token ownership or network/disk access in menu callbacks.
use super::State;
#[path = "device_live_viewer_ui.rs"]
pub(super) mod live;
use crate::device_authorization_config::Action;
use windows::{
    core::{w, Error, Result, HSTRING},
    Win32::{Foundation::*, UI::WindowsAndMessaging::*},
};
#[path = "device_account_projection_ui.rs"]
pub(in crate::desktop) mod account;
#[path = "device_authorization_dialog.rs"]
mod dialog;
#[path = "device_projection_edit_ui.rs"]
pub(super) mod editing;
#[path = "device_projection_policy_ui.rs"]
pub(super) mod policy;
#[path = "device_projection_inbox_ui.rs"]
pub(super) mod projection;
#[path = "device_authorization_recovery.rs"]
mod recovery;
#[cfg(test)]
pub(super) use recovery::RECENT_BASE as TEST_RECENT_RECOVERY;
#[cfg(test)]
pub(super) unsafe fn confirm_recovery_for_test(
    hwnd: HWND,
    state: &State,
    choose: impl FnOnce(&crate::live_relay_http::RecoveryCandidate) -> bool,
) -> Result<()> {
    recovery::confirm_with(hwnd, state, choose)
}
#[path = "device_projection_sender_ui.rs"]
pub(super) mod sender;
#[path = "device_settings_menu.rs"]
pub(super) mod settings;
#[path = "projection_share_ui.rs"]
pub(in crate::desktop) mod share;
#[path = "device_authorization_surface.rs"]
mod surface;
#[path = "device_projection_targets_ui.rs"]
pub(super) mod targets;
pub(super) const SELECT: usize = 0x1091;
pub(super) const PAIR: usize = 0x1092;
pub(super) const APPROVAL: usize = 0x1093;
pub(super) const RENEW: usize = 0x1094;
pub(super) const CANCEL: usize = 0x1095;
pub(super) const DISCOVER: usize = 0x1096;
pub(super) const OPEN_ART: usize = 0x1097;
pub(super) const LOAD_RECOVERY: usize = 0x1098;
pub(super) const RESTORE_SURFACE: usize = 0x1099;
pub(super) const CONFIRM_SURFACE: usize = 0x109a;
pub(super) const RESUME_SURFACE: usize = 0x109b;

pub(super) enum Request {
    AccountProjection(Box<account::Request>),
    WatchLive(live::Selection),
    StopLive(std::sync::Arc<crate::desktop::composition::relay::sessions::ViewerKey>),
    ReceivePolicy(crate::projection_receive_policy::Policy),
    ReceivePause(bool),
    Projection(projection::Request),
    ProjectionTargets,
    SendProjection(sender::Selection),
    ShareProjection(share::SourceSelection),
    SendProjectionGroup(sender::groups::Selection),
    StopProjectionGroup(sender::groups::StopSelection),
    CancelProjectionSend,
    StopProjectionSend(Box<sender::StopSelection>),
    RetryProjectionSend(crate::workspace_id::UnitId),
    SubmitProjectionEdit(editing::Selection),
    ResolveProjectionEdit(editing::Selection, bool),
    SetProjectionEditMode(editing::Selection, crate::projection_edit::Mode),
    CancelProjectionEdit,
    InitializeProjectionEdit(crate::workspace_id::UnitId),
    ResumeSurface(String),
    ConfirmSurface(String),
    Select(String),
    Start(Action),
    Cancel,
    Discover,
    OpenArt { art: String, node: String },
    LoadRecovery(std::path::PathBuf),
    QueryRecovery(std::sync::Arc<crate::surface_recovery_record::Record>),
    RestoreSurface(std::sync::Arc<crate::live_relay_http::RecoveryCandidate>),
}
#[derive(Default)]
pub(super) struct View {
    pub account: account::View,
    pub share: Option<std::rc::Rc<share::Window>>,
    pub share_decode: Option<share::Decode>,
    pub live: live::View,
    pub consent: policy::Consent,
    pub projection: projection::View,
    pub targets: targets::View,
    pub sender: sender::View,
    pub editing: editing::View,
    pub continuation: Option<String>,
    pub confirmation: Option<String>,
    pub origin: Option<String>,
    pub connected: bool,
    pub loom_name: String,
    pub loom_id: Option<String>,
    pub connection_password: zeroize::Zeroizing<String>,
    pub request: Option<Request>,
    pub busy: bool,
    pub closed: bool,
    pub status: String,
    pub catalog_busy: bool,
    pub mount_busy: bool,
    pub catalog: Option<crate::live_relay_catalog_runtime::Catalog>,
    pub recovery: Vec<std::sync::Arc<crate::surface_recovery_record::Record>>,
    pub recent_surface_recovery: Vec<std::sync::Arc<crate::surface_recovery_record::Record>>,
    pub recovery_busy: bool,
    pub restoring: bool,
    pub recovery_status: String,
    pub recovery_candidate: Option<std::sync::Arc<crate::live_relay_http::RecoveryCandidate>>,
}
pub(super) fn handles(command: usize) -> bool {
    matches!(
        command,
        SELECT
            | PAIR
            | APPROVAL
            | RENEW
            | CANCEL
            | DISCOVER
            | OPEN_ART
            | LOAD_RECOVERY
            | RESTORE_SURFACE
            | CONFIRM_SURFACE
            | RESUME_SURFACE
    ) || live::handles(command)
        || policy::handles(command)
        || projection::handles(command)
        || command == targets::REFRESH
        || sender::handles(command)
        || editing::handles(command)
        || recovery::handles(command)
}
pub(super) unsafe fn append_menu(menu: HMENU, state: &State) -> Result<()> {
    let view = state.authorization.borrow();
    append_connection(menu, &view)?;
    append_actions(
        menu,
        &view,
        &[
            (
                DISCOVER,
                crate::ui_text::text("integration-refresh-remote-sessions"),
            ),
            (
                OPEN_ART,
                crate::ui_text::text("integration-open-art-preview"),
            ),
            (
                CONFIRM_SURFACE,
                crate::ui_text::text("integration-confirm-surface-actions"),
            ),
            (
                RESUME_SURFACE,
                crate::ui_text::text("integration-resume-surface-batch"),
            ),
        ],
    )?;
    recovery::append_menu(menu, &view)?;
    live::append_menu(menu, &view)?;
    policy::append_menu(menu, &view)?;
    projection::append_menu(menu, &view)?;
    targets::append_menu(menu, &view, &state.unit_id)?;
    editing::append_menu(menu, state, &view)?;
    AppendMenuW(menu, MF_SEPARATOR, 0, None)
}
unsafe fn append_connection(menu: HMENU, view: &View) -> Result<()> {
    let label = match &view.origin {
        Some(origin) => format!("Loom：{}", origin.replace('&', "&&")),
        None => crate::ui_text::text("integration-no-loom").into(),
    };
    AppendMenuW(menu, MF_SEPARATOR, 0, None)?;
    AppendMenuW(menu, MF_STRING | MF_GRAYED, 0, &HSTRING::from(label))?;
    if !view.status.is_empty() {
        AppendMenuW(menu, MF_STRING | MF_GRAYED, 0, &HSTRING::from(&view.status))?;
    }
    append_actions(
        menu,
        view,
        &[
            (SELECT, crate::ui_text::text("integration-choose-loom")),
            (PAIR, crate::ui_text::text("integration-submit-pairing")),
            (APPROVAL, crate::ui_text::text("integration-wait-admin")),
            (RENEW, crate::ui_text::text("integration-get-renew-session")),
            (
                CANCEL,
                crate::ui_text::text("integration-cancel-clear-session"),
            ),
        ],
    )
}
unsafe fn append_actions(menu: HMENU, view: &View, actions: &[(usize, &str)]) -> Result<()> {
    for &(id, label) in actions {
        let enabled = !view.closed
            && (id != CONFIRM_SURFACE || view.confirmation.is_some())
            && (id != RESUME_SURFACE || view.continuation.is_some())
            && (id != DISCOVER || !view.catalog_busy)
            && (id != OPEN_ART || !view.mount_busy)
            && view.request.is_none()
            && (id == SELECT || (view.origin.is_some() && (id == CANCEL || !view.busy)));
        AppendMenuW(
            menu,
            MF_STRING | if enabled { MF_ENABLED } else { MF_GRAYED },
            id,
            &HSTRING::from(label),
        )?;
    }
    Ok(())
}
pub(super) unsafe fn execute(hwnd: HWND, state: &State, command: usize) -> Result<()> {
    let result = if live::handles(command) {
        live::execute(hwnd, state, command)
    } else if editing::handles(command) {
        editing::execute(hwnd, state, command)
    } else if sender::handles(command) {
        sender::execute(hwnd, state, command)
    } else if policy::handles(command) {
        let request = policy::request(&state.authorization.borrow(), command)?;
        queue(hwnd, state, request)
    } else if projection::handles(command) {
        projection::execute(hwnd, state, command)
    } else if command == targets::REFRESH {
        let request = targets::request(&state.authorization.borrow())?;
        queue(hwnd, state, request)
    } else if command == LOAD_RECOVERY {
        recovery::choose(hwnd, state)
    } else if command == RESTORE_SURFACE {
        recovery::confirm(hwnd, state)
    } else if recovery::handles(command) {
        recovery::query(hwnd, state, command)
    } else if command == OPEN_ART {
        surface::choose(hwnd, state, || dialog::show_art(hwnd))
    } else if command == SELECT {
        let mut password = zeroize::Zeroizing::new(String::new());
        let result = select_with(hwnd, state, |old| {
            dialog::show(hwnd, old).map(|(origin, secret)| {
                password = secret;
                origin
            })
        });
        if result.is_ok()
            && matches!(
                state.authorization.borrow().request,
                Some(Request::Select(_))
            )
        {
            state.authorization.borrow_mut().connection_password = password;
        }
        result
    } else {
        let request = match command {
            PAIR => Request::Start(Action::RequestPairing),
            APPROVAL => Request::Start(Action::AwaitApproval),
            RENEW => Request::Start(Action::Renew),
            CANCEL => Request::Cancel,
            DISCOVER => Request::Discover,
            RESUME_SURFACE => Request::ResumeSurface(
                state
                    .authorization
                    .borrow()
                    .continuation
                    .clone()
                    .ok_or_else(|| Error::new(E_ABORT, "No pending Surface continuation"))?,
            ),
            CONFIRM_SURFACE => Request::ConfirmSurface(
                state
                    .authorization
                    .borrow()
                    .confirmation
                    .clone()
                    .ok_or_else(|| Error::new(E_ABORT, "No pending Surface confirmation"))?,
            ),
            _ => return Err(Error::new(E_INVALIDARG, "Unknown authorization command")),
        };
        queue(hwnd, state, request)
    };
    if let Err(error) = result {
        MessageBoxW(
            Some(hwnd),
            &HSTRING::from(crate::ui_text::format(
                "integration-authorization-action-failed",
                &[("error", error.to_string().into())],
            )),
            w!("Otter"),
            MB_OK | MB_ICONWARNING,
        );
    }
    Ok(())
}
unsafe fn select_with(
    hwnd: HWND,
    state: &State,
    choose: impl FnOnce(&str) -> Option<String>,
) -> Result<()> {
    if state.modals.blocked() || state.exit_requested.get() {
        return Ok(());
    }
    if !IsWindow(Some(hwnd)).as_bool() {
        return Err(Error::new(E_ABORT, "Authorization requester closed"));
    }
    let old = state.authorization.borrow().origin.clone();
    let flag = state.file_dialog.clone();
    let guard = super::file_dialog::Guard::begin(&flag)
        .ok_or_else(|| Error::new(E_ABORT, "Another dialog is active"))?;
    let selected = choose(old.as_deref().unwrap_or(""));
    drop(guard);
    let Some(selected) = selected else {
        return Ok(());
    };
    if state.authorization.borrow().origin != old {
        return Err(Error::new(
            E_ABORT,
            "Authorization selection changed while editing",
        ));
    }
    let origin = crate::device_session_attempt::canonical_origin(&selected).map_err(|_| {
        Error::new(
            E_INVALIDARG,
            "Expected an HTTPS origin without credentials or path",
        )
    })?;
    queue(hwnd, state, Request::Select(origin))
}
unsafe fn queue(hwnd: HWND, state: &State, request: Request) -> Result<()> {
    if !IsWindow(Some(hwnd)).as_bool() || state.exit_requested.get() || state.modals.blocked() {
        return Err(Error::new(E_ABORT, "Authorization requester unavailable"));
    }
    let mut view = state.authorization.borrow_mut();
    if view.closed
        || view.request.is_some()
        || (matches!(request, Request::Start(_)) && view.busy)
        || (matches!(request, Request::Discover) && (view.busy || view.catalog_busy))
        || (matches!(request, Request::Projection(_) | Request::ProjectionTargets)
            && (view.busy || view.projection.busy || view.targets.busy))
        || (matches!(request, Request::OpenArt { .. }) && (view.busy || view.mount_busy))
        || (matches!(request, Request::RestoreSurface(_))
            && (view.busy || view.mount_busy || view.recovery_busy))
        || (matches!(
            request,
            Request::LoadRecovery(_) | Request::QueryRecovery(_)
        ) && (view.busy || view.recovery_busy || view.mount_busy))
        || (!matches!(
            request,
            Request::Select(_)
                | Request::LoadRecovery(_)
                | Request::ReceivePolicy(_)
                | Request::ReceivePause(_)
        ) && view.origin.is_none())
    {
        return Err(Error::new(E_ABORT, "Authorization request unavailable"));
    }
    view.request = Some(request);
    // Thread messages survive initiating-window destruction; roll back failures rather than leaving permanently unconsumed busy requests.
    if let Err(error) = PostMessageW(None, WM_NULL, WPARAM(0), LPARAM(0)) {
        view.request = None;
        return Err(error);
    }
    Ok(())
}

#[cfg(test)]
#[path = "device_authorization_ui_tests.rs"]
mod tests;

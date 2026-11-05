//! 所有工作窗口共享授权请求和展示状态；不持有令牌，不在菜单回调中访问网络或磁盘。
use super::State;
#[path = "device_live_viewer_ui.rs"]
pub(super) mod live;
use crate::device_authorization_config::Action;
use windows::{
    core::{w, Error, Result, HSTRING},
    Win32::{Foundation::*, UI::WindowsAndMessaging::*},
};
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
#[path = "device_projection_sender_ui.rs"]
pub(super) mod sender;
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
const RECOVERY_BASE: usize = 0x10c0;

pub(super) enum Request {
    WatchLive(live::Selection),
    StopLive(std::sync::Arc<crate::desktop::composition::relay::sessions::ViewerKey>),
    ReceivePolicy(crate::projection_receive_policy::Policy),
    ReceivePause(bool),
    Projection(projection::Request),
    ProjectionTargets,
    SendProjection(sender::Selection),
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
    pub live: live::View,
    pub consent: policy::Consent,
    pub projection: projection::View,
    pub targets: targets::View,
    pub sender: sender::View,
    pub editing: editing::View,
    pub continuation: Option<String>,
    pub confirmation: Option<String>,
    pub origin: Option<String>,
    pub request: Option<Request>,
    pub busy: bool,
    pub closed: bool,
    pub status: String,
    pub catalog_busy: bool,
    pub mount_busy: bool,
    pub catalog: Option<crate::live_relay_catalog_runtime::Catalog>,
    pub recovery: Vec<std::sync::Arc<crate::surface_recovery_record::Record>>,
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
        || (RECOVERY_BASE..RECOVERY_BASE + crate::surface_recovery_record::MAX_RECORDS)
            .contains(&command)
}
pub(super) unsafe fn append_menu(menu: HMENU, state: &State) -> Result<()> {
    let view = state.authorization.borrow();
    let label = match &view.origin {
        Some(origin) => format!("Loom：{}", origin.replace('&', "&&")),
        None => "Loom：未选择服务".into(),
    };
    AppendMenuW(menu, MF_SEPARATOR, 0, None)?;
    AppendMenuW(menu, MF_STRING | MF_GRAYED, 0, &HSTRING::from(label))?;
    if !view.status.is_empty() {
        AppendMenuW(menu, MF_STRING | MF_GRAYED, 0, &HSTRING::from(&view.status))?;
    }
    for (id, label) in [
        (SELECT, "选择 Loom 服务…"),
        (PAIR, "提交设备配对"),
        (APPROVAL, "等待管理员批准"),
        (RENEW, "获取 / 续签授权会话"),
        (CANCEL, "取消请求并清除本地会话"),
        (DISCOVER, "刷新远程会话目录"),
        (OPEN_ART, "打开 Art（原生预览）…"),
        (CONFIRM_SURFACE, "处理待确认的 Surface 操作…"),
        (RESUME_SURFACE, "继续 Surface 批次的剩余操作"),
    ] {
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
    recovery::append_menu(menu, &view)?;
    live::append_menu(menu, &view)?;
    policy::append_menu(menu, &view)?;
    projection::append_menu(menu, &view)?;
    targets::append_menu(menu, &view, &state.unit_id)?;
    editing::append_menu(menu, state, &view)?;
    AppendMenuW(menu, MF_SEPARATOR, 0, None)
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
    } else if (RECOVERY_BASE..RECOVERY_BASE + crate::surface_recovery_record::MAX_RECORDS)
        .contains(&command)
    {
        let record = state
            .authorization
            .borrow()
            .recovery
            .get(command - RECOVERY_BASE)
            .cloned();
        match record {
            Some(record) => queue(hwnd, state, Request::QueryRecovery(record)),
            None => Err(Error::new(E_ABORT, "Recovery selection expired")),
        }
    } else if command == OPEN_ART {
        surface::choose(hwnd, state, || dialog::show_art(hwnd))
    } else if command == SELECT {
        select_with(hwnd, state, |old| dialog::show(hwnd, old))
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
            &HSTRING::from(format!("授权操作未执行。\n{error}")),
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
    // 线程消息不随发起窗口销毁而丢失；失败回滚，不能遗留永远不消费的 busy 请求。
    if let Err(error) = PostMessageW(None, WM_NULL, WPARAM(0), LPARAM(0)) {
        view.request = None;
        return Err(error);
    }
    Ok(())
}

#[cfg(test)]
#[path = "device_authorization_ui_tests.rs"]
mod tests;

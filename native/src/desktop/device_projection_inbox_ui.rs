//! 传统投送的有界菜单数据和显式请求；不持凭据、不联网、不冒充保存或呈现。
use super::{Request as AuthorizationRequest, View as AuthorizationView};
use crate::projection_delivery::{Invitation, MAX_INVITATIONS};
use std::sync::Arc;
use windows::{
    core::{w, Error, Result, HSTRING},
    Win32::{Foundation::E_ABORT, UI::WindowsAndMessaging::*},
};
pub(in crate::desktop) const REFRESH: usize = 0x1300;
pub(in crate::desktop) const REJECT_BASE: usize = 0x1340;
pub(in crate::desktop) const ACCEPT_BASE: usize = 0x1400;
pub(in crate::desktop) const REATTACH_BASE: usize = 0x1500;
#[path = "device_projection_accept_ui.rs"]
pub(in crate::desktop) mod accept;
#[path = "device_projection_recovery_ui.rs"]
pub(in crate::desktop) mod recovery;
pub(in crate::desktop) enum Request {
    Refresh,
    Reject(Arc<Invitation>),
    Accept(Arc<Invitation>),
    Reattach(Arc<Invitation>),
    Compensate(Arc<crate::projection_delivery::journal::Ticket>),
    Display(Arc<crate::projection_delivery::journal::Ticket>),
}
#[derive(Default)]
pub(in crate::desktop) struct View {
    pub entries: Vec<Arc<Invitation>>,
    pub busy: bool,
    pub status: String,
    pub recovery: recovery::View,
    pub participation: Option<super::policy::Participation>,
}
pub(super) fn handles(command: usize) -> bool {
    command == REFRESH
        || (REJECT_BASE..REJECT_BASE + MAX_INVITATIONS).contains(&command)
        || (ACCEPT_BASE..ACCEPT_BASE + MAX_INVITATIONS).contains(&command)
        || (REATTACH_BASE..REATTACH_BASE + MAX_INVITATIONS).contains(&command)
        || recovery::handles(command)
        || recovery::handles_display(command)
}
fn available(view: &AuthorizationView) -> bool {
    !view.closed
        && !view.busy
        && !view.projection.busy
        && !view.targets.busy
        && view.request.is_none()
        && view.origin.is_some()
        && view.consent.participation().is_some()
}
pub(super) fn request(view: &AuthorizationView, command: usize) -> Result<Request> {
    if recovery::handles_display(command) {
        return recovery::display_request(view, command).map(Request::Display);
    }
    if recovery::handles(command) {
        return recovery::request(view, command).map(Request::Compensate);
    }
    if !available(view) {
        return Err(Error::new(E_ABORT, "Projection inbox unavailable"));
    }
    if command == REFRESH {
        return Ok(Request::Refresh);
    }
    let accepting = (ACCEPT_BASE..ACCEPT_BASE + MAX_INVITATIONS).contains(&command);
    let reattaching = (REATTACH_BASE..REATTACH_BASE + MAX_INVITATIONS).contains(&command);
    if accepting && view.consent.stamp().is_none() {
        return Err(Error::new(E_ABORT, "Receive policy disallows new pushes"));
    }
    let entry = command
        .checked_sub(if reattaching {
            REATTACH_BASE
        } else if accepting {
            ACCEPT_BASE
        } else {
            REJECT_BASE
        })
        .filter(|index| *index < MAX_INVITATIONS)
        .and_then(|index| view.projection.entries.get(index))
        .filter(|entry| {
            entry.accepted() == reattaching && view.origin.as_deref() == Some(entry.origin())
        })
        .cloned()
        .ok_or_else(|| Error::new(E_ABORT, "Projection selection expired"))?;
    Ok(if reattaching {
        Request::Reattach(entry)
    } else if accepting {
        Request::Accept(entry)
    } else {
        Request::Reject(entry)
    })
}
pub(in crate::desktop) fn reset(view: &mut AuthorizationView) {
    view.projection = View::default();
    if matches!(view.request, Some(AuthorizationRequest::Projection(_))) {
        view.request = None;
    }
}
pub(super) unsafe fn append_menu(menu: HMENU, view: &AuthorizationView) -> Result<()> {
    let enabled = available(view);
    AppendMenuW(menu, MF_SEPARATOR, 0, None)?;
    AppendMenuW(
        menu,
        MF_STRING | if enabled { MF_ENABLED } else { MF_GRAYED },
        REFRESH,
        w!("刷新投送收件箱"),
    )?;
    if !view.projection.status.is_empty() {
        AppendMenuW(
            menu,
            MF_STRING | MF_GRAYED,
            0,
            &HSTRING::from(view.projection.status.replace('&', "&&")),
        )?;
    }
    for (index, entry) in view
        .projection
        .entries
        .iter()
        .enumerate()
        .take(MAX_INVITATIONS)
    {
        let active = enabled && view.origin.as_deref() == Some(entry.origin());
        let name: String = entry.source_name().chars().take(64).collect();
        let suffix = if entry.source_name().chars().count() > 64 {
            "…"
        } else {
            ""
        };
        let label = format!(
            "{}{}：{name}{suffix} · {}",
            if entry.peer_id().is_some() {
                "离线 · "
            } else {
                ""
            },
            if entry.accepted() {
                "恢复已接受投送"
            } else {
                "拒绝投送"
            },
            &entry.envelope().projection_id()[11..19]
        )
        .replace('&', "&&");
        if !entry.accepted() {
            AppendMenuW(
                menu,
                MF_STRING
                    | if active && view.consent.stamp().is_some() {
                        MF_ENABLED
                    } else {
                        MF_GRAYED
                    },
                ACCEPT_BASE + index,
                &HSTRING::from(
                    format!(
                        "{}接受投送：{name}{suffix} · {}",
                        if entry.peer_id().is_some() {
                            "离线 · "
                        } else {
                            ""
                        },
                        &entry.envelope().projection_id()[11..19]
                    )
                    .replace('&', "&&"),
                ),
            )?;
        }
        AppendMenuW(
            menu,
            MF_STRING | if active { MF_ENABLED } else { MF_GRAYED },
            if entry.accepted() {
                REATTACH_BASE
            } else {
                REJECT_BASE
            } + index,
            &HSTRING::from(label),
        )?;
    }
    recovery::append_menu(menu, view)
}
pub(super) unsafe fn execute(
    hwnd: windows::Win32::Foundation::HWND,
    state: &super::State,
    command: usize,
) -> Result<()> {
    if recovery::handles(command) {
        recovery::confirm(hwnd, state, command)
    } else if (ACCEPT_BASE..ACCEPT_BASE + MAX_INVITATIONS).contains(&command)
        || (REATTACH_BASE..REATTACH_BASE + MAX_INVITATIONS).contains(&command)
    {
        accept::confirm(hwnd, state, command)
    } else {
        let request = request(&state.authorization.borrow(), command)?;
        super::queue(hwnd, state, AuthorizationRequest::Projection(request))
    }
}

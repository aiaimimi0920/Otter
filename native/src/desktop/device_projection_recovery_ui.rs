//! 未结投送的显式解绑/已保存回执恢复；不持凭据，不自动恢复权限或显示隐藏图。
use super::*;
use crate::{
    desktop::State,
    projection_delivery::journal::{Ticket, MAX_ENTRIES},
};
use windows::Win32::Foundation::{E_ABORT, HWND};
pub(in crate::desktop) const BASE: usize = 0x1380;
pub(in crate::desktop) const DISPLAY_BASE: usize = 0x1480;
#[derive(Default)]
pub(in crate::desktop) struct View {
    pub entries: Vec<Arc<Ticket>>,
    pub saved: Vec<Arc<Ticket>>,
    pub status: String,
}
pub(super) fn handles(command: usize) -> bool {
    (BASE..BASE + MAX_ENTRIES).contains(&command)
}
pub(super) fn handles_display(command: usize) -> bool {
    (DISPLAY_BASE..DISPLAY_BASE + MAX_ENTRIES).contains(&command)
}
pub(super) fn display_request(view: &AuthorizationView, command: usize) -> Result<Arc<Ticket>> {
    let index = command
        .checked_sub(DISPLAY_BASE)
        .filter(|index| *index < MAX_ENTRIES)
        .ok_or_else(|| Error::new(E_ABORT, "Projection recovery selection expired"))?;
    let ticket = request(view, BASE + index)?;
    if ticket.cleanup_pending()
        || !view
            .projection
            .recovery
            .saved
            .iter()
            .any(|old| Arc::ptr_eq(old, &ticket))
    {
        return Err(Error::new(
            E_ABORT,
            "Projection is not a saved recovery candidate",
        ));
    }
    Ok(ticket)
}
pub(super) fn request(view: &AuthorizationView, command: usize) -> Result<Arc<Ticket>> {
    if !available(view) {
        return Err(Error::new(E_ABORT, "Projection recovery unavailable"));
    }
    command
        .checked_sub(BASE)
        .filter(|index| *index < MAX_ENTRIES)
        .and_then(|index| view.projection.recovery.entries.get(index))
        .filter(|entry| view.origin.as_deref() == Some(entry.origin()))
        .cloned()
        .ok_or_else(|| Error::new(E_ABORT, "Projection recovery selection expired"))
}
pub(super) unsafe fn append_menu(menu: HMENU, view: &AuthorizationView) -> Result<()> {
    if !view.projection.recovery.status.is_empty() {
        AppendMenuW(
            menu,
            MF_STRING | MF_GRAYED,
            0,
            &HSTRING::from(&view.projection.recovery.status),
        )?;
    }
    for (index, entry) in view
        .projection
        .recovery
        .entries
        .iter()
        .enumerate()
        .take(MAX_ENTRIES)
    {
        if view.origin.as_deref() != Some(entry.origin()) {
            continue;
        }
        let label = format!(
            "解除{}投送绑定：{}",
            if view
                .projection
                .recovery
                .saved
                .iter()
                .any(|saved| Arc::ptr_eq(saved, entry))
            {
                "已保存"
            } else {
                "未结"
            },
            &entry.envelope().projection_id()[11..19]
        );
        AppendMenuW(
            menu,
            MF_STRING
                | if available(view) {
                    MF_ENABLED
                } else {
                    MF_GRAYED
                },
            BASE + index,
            &HSTRING::from(label),
        )?;
        if !entry.cleanup_pending()
            && view
                .projection
                .recovery
                .saved
                .iter()
                .any(|old| Arc::ptr_eq(old, entry))
        {
            AppendMenuW(
                menu,
                MF_STRING
                    | if available(view) {
                        MF_ENABLED
                    } else {
                        MF_GRAYED
                    },
                DISPLAY_BASE + index,
                &HSTRING::from(format!(
                    "重试呈现回执：{}",
                    &entry.envelope().projection_id()[11..19]
                )),
            )?;
        }
    }
    Ok(())
}
pub(super) unsafe fn confirm(hwnd: HWND, state: &State, command: usize) -> Result<()> {
    confirm_with(hwnd, state, command, |entry| {
        MessageBoxW(
            Some(hwnd),
            &HSTRING::from(format!(
                "解除此投送的远端绑定？此操作不能恢复，本地图片保留。\n服务：{}\n投送：{}",
                entry.envelope().origin(),
                entry.envelope().projection_id()
            )),
            w!("解除投送绑定"),
            MB_YESNO | MB_ICONWARNING | MB_DEFBUTTON2,
        ) == IDYES
    })
}
pub(in crate::desktop) unsafe fn confirm_with(
    hwnd: HWND,
    state: &State,
    command: usize,
    choose: impl FnOnce(&Ticket) -> bool,
) -> Result<()> {
    if state.modals.blocked() || state.exit_requested.get() {
        return Ok(());
    }
    if !IsWindow(Some(hwnd)).as_bool() {
        return Err(Error::new(E_ABORT, "Projection requester closed"));
    }
    let entry = request(&state.authorization.borrow(), command)?;
    let flag = state.file_dialog.clone();
    let guard = crate::desktop::file_dialog::Guard::begin(&flag)
        .ok_or_else(|| Error::new(E_ABORT, "Another dialog is active"))?;
    let accepted = choose(&entry);
    drop(guard);
    if !accepted {
        return Ok(());
    }
    let current = request(&state.authorization.borrow(), command)?;
    if !Arc::ptr_eq(&entry, &current) {
        return Err(Error::new(
            E_ABORT,
            "Projection recovery changed during confirmation",
        ));
    }
    super::super::queue(
        hwnd,
        state,
        AuthorizationRequest::Projection(Request::Compensate(entry)),
    )
}

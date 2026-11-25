//! Explicit risk-acceptance confirmation; preserve the same invitation/initiator before and after, without shared View borrows across native modal loops.
use super::*;
use crate::desktop::State;
use windows::Win32::Foundation::HWND;

pub(super) unsafe fn confirm(hwnd: HWND, state: &State, command: usize) -> Result<()> {
    confirm_with(hwnd, state, command, |entry| {
        MessageBoxW(
            Some(hwnd),
            &HSTRING::from(crate::ui_text::format(
                "integration-accept-delivery-confirm",
                &[
                    (
                        "accepted",
                        if entry.accepted() { "yes" } else { "no" }.into(),
                    ),
                    ("source", entry.source_name().into()),
                    ("origin", entry.envelope().origin().into()),
                    ("id", entry.envelope().projection_id().into()),
                ],
            )),
            &windows::core::HSTRING::from(crate::ui_text::text("integration-accept-delivery")),
            MB_YESNO | MB_ICONQUESTION | MB_DEFBUTTON2,
        ) == IDYES
    })
}
pub(in crate::desktop) unsafe fn confirm_with(
    hwnd: HWND,
    state: &State,
    command: usize,
    choose: impl FnOnce(&Invitation) -> bool,
) -> Result<()> {
    let live = || {
        IsWindow(Some(hwnd)).as_bool()
            && GetWindowLongPtrW(hwnd, GWLP_USERDATA) == (state as *const State) as isize
    };
    if state.modals.blocked() || state.exit_requested.get() || !live() {
        return Err(Error::new(E_ABORT, "Projection requester unavailable"));
    }
    let selected = request(&state.authorization.borrow(), command)?;
    let entry = match &selected {
        Request::Accept(entry) | Request::Reattach(entry) => entry,
        _ => return Err(Error::new(E_ABORT, "Projection accept selection required")),
    };
    let flag = state.file_dialog.clone();
    let guard = crate::desktop::file_dialog::Guard::begin(&flag)
        .ok_or_else(|| Error::new(E_ABORT, "Another dialog is active"))?;
    let confirmed = choose(entry);
    drop(guard);
    if !confirmed {
        return Ok(());
    }
    let current = match request(&state.authorization.borrow(), command)? {
        Request::Accept(entry) | Request::Reattach(entry) => entry,
        _ => return Err(Error::new(E_ABORT, "Projection selection changed")),
    };
    if !Arc::ptr_eq(entry, &current)
        || !live()
        || state.modals.blocked()
        || state.exit_requested.get()
    {
        return Err(Error::new(
            E_ABORT,
            "Projection changed during confirmation",
        ));
    }
    super::super::queue(hwnd, state, AuthorizationRequest::Projection(selected))
}

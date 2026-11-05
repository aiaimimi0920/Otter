//! 显式接受风险确认；前后保同一邀请/发起窗口，不跨原生模态循环借用共享 View。
use super::*;
use crate::desktop::State;
use windows::Win32::Foundation::HWND;

pub(super) unsafe fn confirm(hwnd: HWND, state: &State, command: usize) -> Result<()> {
    confirm_with(hwnd, state, command, |entry| {
        MessageBoxW(
            Some(hwnd),
            &HSTRING::from(format!(
                "{}并保存到当前工作区？\n来源：{}\n服务：{}\n投送：{}",
                if entry.accepted() {
                    "只读恢复已接受的投送"
                } else {
                    "接受此投送"
                },
                entry.source_name(),
                entry.envelope().origin(),
                entry.envelope().projection_id()
            )),
            w!("接受投送"),
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

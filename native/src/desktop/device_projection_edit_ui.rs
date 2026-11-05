//! 复用原文档菜单/请求单槽；只固定Unit和会话Stamp，不在回调里发网、写盘或复制栅格。
use super::*;
pub(in crate::desktop) const SUBMIT: usize = 0x1780;
pub(in crate::desktop) const CANCEL: usize = 0x1781;
pub(in crate::desktop) const KEEP_LOCAL: usize = 0x1782;
pub(in crate::desktop) const USE_REMOTE: usize = 0x1783;
pub(in crate::desktop) const INITIALIZE: usize = 0x1784;
pub(in crate::desktop) const ONE_WAY: usize = 0x1785;
pub(in crate::desktop) const TWO_WAY: usize = 0x1786;
pub(in crate::desktop) struct Selection {
    pub unit: crate::workspace_id::UnitId,
    pub stamp: crate::desktop::projection_edit_workspace::Stamp,
}
#[derive(Default)]
pub(in crate::desktop) struct View {
    pub busy: bool,
    pub unit: Option<crate::workspace_id::UnitId>,
    pub status: String,
}
pub(in crate::desktop) fn handles(command: usize) -> bool {
    matches!(
        command,
        SUBMIT | CANCEL | KEEP_LOCAL | USE_REMOTE | INITIALIZE | ONE_WAY | TWO_WAY
    )
}
fn can_initialize(state: &State, view: &super::View) -> bool {
    !view.closed
        && !view.busy
        && !view.editing.busy
        && !view.sender.busy
        && view.sender.ready
        && view.request.is_none()
        && super::sender::stops(view, &state.unit_id).any(|binding| {
            !binding.stopped() && Some(binding.envelope().origin()) == view.origin.as_deref()
        })
        && state.projection_edit.borrow().as_ref().is_none_or(|owner| {
            let snapshot = owner.capture(&state.canvas.borrow().editor);
            snapshot.role() == crate::projection_edit::workspace::Role::Source
                && snapshot.suspended().is_none()
        })
}
fn selectable(state: &State, view: &super::View) -> bool {
    !view.closed
        && !view.busy
        && !view.editing.busy
        && view.request.is_none()
        && state
            .projection_edit
            .borrow()
            .as_ref()
            .is_some_and(|owner| {
                let snapshot = owner.capture(&state.canvas.borrow().editor);
                snapshot.suspended().is_none()
                    && Some(snapshot.context().origin()) == view.origin.as_deref()
            })
}
pub(super) unsafe fn append_menu(menu: HMENU, state: &State, view: &super::View) -> Result<()> {
    if super::sender::stops(view, &state.unit_id).any(|binding| !binding.stopped()) {
        AppendMenuW(
            menu,
            MF_STRING
                | if can_initialize(state, view) {
                    MF_ENABLED
                } else {
                    MF_GRAYED
                },
            INITIALIZE,
            w!("启用 / 恢复源端协作标注"),
        )?;
    }
    if state.projection_edit.borrow().is_none() {
        return append_status(menu, state, view);
    }
    AppendMenuW(
        menu,
        MF_STRING
            | if selectable(state, view) {
                MF_ENABLED
            } else {
                MF_GRAYED
            },
        SUBMIT,
        w!("提交 / 重试协作修改"),
    )?;
    let snapshot = state
        .projection_edit
        .borrow()
        .as_ref()
        .unwrap()
        .capture(&state.canvas.borrow().editor);
    for (id, mode, label) in [
        (ONE_WAY, crate::projection_edit::Mode::OneWay, "单向修改"),
        (TWO_WAY, crate::projection_edit::Mode::TwoWay, "双向修改"),
    ] {
        let enabled = selectable(state, view)
            && snapshot.role() == crate::projection_edit::workspace::Role::Source
            && snapshot.document().mode() != mode;
        AppendMenuW(
            menu,
            MF_STRING
                | if snapshot.document().mode() == mode {
                    MF_CHECKED
                } else {
                    MF_UNCHECKED
                }
                | if enabled { MF_ENABLED } else { MF_GRAYED },
            id,
            &HSTRING::from(label),
        )?;
    }
    if snapshot.conflict().is_some() {
        for (id, label) in [
            (KEEP_LOCAL, "保留本地协作修改"),
            (USE_REMOTE, "采用远端协作标注"),
        ] {
            let enabled = selectable(state, view)
                && (id != KEEP_LOCAL
                    || snapshot.role() == crate::projection_edit::workspace::Role::Source
                    || snapshot.document().mode() == crate::projection_edit::Mode::TwoWay);
            AppendMenuW(
                menu,
                MF_STRING | if enabled { MF_ENABLED } else { MF_GRAYED },
                id,
                &HSTRING::from(label),
            )?;
        }
    }
    append_status(menu, state, view)
}
unsafe fn append_status(menu: HMENU, state: &State, view: &super::View) -> Result<()> {
    if view.editing.unit.as_ref() == Some(&state.unit_id) {
        if view.editing.busy {
            AppendMenuW(menu, MF_STRING, CANCEL, w!("取消当前协作提交"))?;
        }
        if !view.editing.status.is_empty() {
            AppendMenuW(
                menu,
                MF_STRING | MF_GRAYED,
                0,
                &HSTRING::from(view.editing.status.replace('&', "&&")),
            )?;
        }
    }
    Ok(())
}
pub(super) unsafe fn execute(hwnd: HWND, state: &State, command: usize) -> Result<()> {
    if command == INITIALIZE {
        if !can_initialize(state, &state.authorization.borrow()) {
            return Err(Error::new(
                E_ABORT,
                "Source collaboration selection expired",
            ));
        }
        return queue(
            hwnd,
            state,
            Request::InitializeProjectionEdit(state.unit_id.clone()),
        );
    }
    let request = if command == CANCEL {
        if !state.authorization.borrow().editing.busy {
            return Err(Error::new(E_ABORT, "No pending collaboration submission"));
        }
        Request::CancelProjectionEdit
    } else {
        if !selectable(state, &state.authorization.borrow()) {
            return Err(Error::new(E_ABORT, "Collaboration selection expired"));
        }
        let selection = Selection {
            unit: state.unit_id.clone(),
            stamp: state.projection_edit.borrow().as_ref().unwrap().stamp(),
        };
        if matches!(command, ONE_WAY | TWO_WAY) {
            let snapshot = state
                .projection_edit
                .borrow()
                .as_ref()
                .unwrap()
                .capture(&state.canvas.borrow().editor);
            let mode = if command == ONE_WAY {
                crate::projection_edit::Mode::OneWay
            } else {
                crate::projection_edit::Mode::TwoWay
            };
            if snapshot.role() != crate::projection_edit::workspace::Role::Source
                || snapshot.document().mode() == mode
            {
                return Err(Error::new(E_ABORT, "Collaboration mode selection expired"));
            }
            Request::SetProjectionEditMode(selection, mode)
        } else if matches!(command, KEEP_LOCAL | USE_REMOTE) {
            let snapshot = state
                .projection_edit
                .borrow()
                .as_ref()
                .unwrap()
                .capture(&state.canvas.borrow().editor);
            if snapshot.conflict().is_none()
                || (command == KEEP_LOCAL
                    && snapshot.role() != crate::projection_edit::workspace::Role::Source
                    && snapshot.document().mode() != crate::projection_edit::Mode::TwoWay)
            {
                return Err(Error::new(
                    E_ABORT,
                    "Collaboration conflict selection expired",
                ));
            }
            Request::ResolveProjectionEdit(selection, command == KEEP_LOCAL)
        } else {
            Request::SubmitProjectionEdit(selection)
        }
    };
    queue(hwnd, state, request)
}

//! Live system menus enqueue only bounded publish/stop intents; networking and frame readback stay with Application background owners.
use super::*;
pub(in crate::desktop) const PUBLISH_DIRECT: usize = 0x2f80;
pub(in crate::desktop) const PUBLISH: usize = 0x2fb0;
pub(in crate::desktop) const STOP: usize = 0x2fc0;
pub(in crate::desktop) const PUBLISH_CONTROL: usize = 0x2fa0;
pub(in crate::desktop) const PUBLISH_OBSERVE: usize = 0x2f90;
pub(super) fn handles(command: usize) -> bool {
    matches!(
        command,
        PUBLISH | PUBLISH_DIRECT | PUBLISH_CONTROL | PUBLISH_OBSERVE | STOP
    )
}
#[derive(Clone, Copy)]
pub(in crate::desktop) enum Request {
    Publish,
    PublishDirect,
    PublishInteractive,
    PublishObservations,
    Stop,
}
#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub(in crate::desktop) enum Phase {
    #[default]
    Idle,
    Starting,
    Publishing,
    Recovering,
    Stopping,
    Failed,
    Unconfirmed,
}
#[derive(Default)]
pub(super) struct Control {
    phase: Cell<Phase>,
    request: Cell<Option<Request>>,
}
pub(super) unsafe fn install(hwnd: HWND) -> Result<()> {
    let menu = GetSystemMenu(hwnd, false);
    AppendMenuW(
        menu,
        MF_STRING,
        PUBLISH_DIRECT,
        &windows::core::HSTRING::from(crate::ui_text::text("integration-project-live")),
    )?;
    AppendMenuW(
        menu,
        MF_STRING,
        PUBLISH,
        &windows::core::HSTRING::from(crate::ui_text::text("integration-publish-live")),
    )?;
    AppendMenuW(
        menu,
        MF_STRING | MF_GRAYED,
        PUBLISH_CONTROL,
        &windows::core::HSTRING::from(crate::ui_text::text("integration-publish-live-control")),
    )?;
    AppendMenuW(
        menu,
        MF_STRING,
        STOP,
        &windows::core::HSTRING::from(crate::ui_text::text("integration-stop-live-publication")),
    )?;
    AppendMenuW(
        menu,
        MF_STRING | MF_GRAYED,
        PUBLISH_OBSERVE,
        &windows::core::HSTRING::from(crate::ui_text::text("integration-publish-live-observation")),
    )?;
    let _ = EnableMenuItem(menu, STOP as u32, MF_BYCOMMAND | MF_GRAYED);
    Ok(())
}
pub(super) fn queue(state: &State, command: usize) {
    let phase = state.publication.phase.get();
    if matches!(
        command,
        PUBLISH | PUBLISH_DIRECT | PUBLISH_CONTROL | PUBLISH_OBSERVE
    ) && matches!(phase, Phase::Idle | Phase::Failed | Phase::Unconfirmed)
    {
        state
            .publication
            .request
            .set(Some(if command == PUBLISH_CONTROL {
                Request::PublishInteractive
            } else if command == PUBLISH_OBSERVE {
                Request::PublishObservations
            } else if command == PUBLISH_DIRECT {
                Request::PublishDirect
            } else {
                Request::Publish
            }));
    } else if command == STOP
        && matches!(
            phase,
            Phase::Starting | Phase::Publishing | Phase::Recovering | Phase::Stopping
        )
    {
        state.publication.request.set(Some(Request::Stop));
    }
}
impl LiveWindow {
    pub(in crate::desktop) fn persistence_state(&self) -> Option<(RECT, f32)> {
        let mut rect = RECT::default();
        (self.is_alive() && unsafe { GetWindowRect(self.hwnd, &mut rect) }.is_ok())
            .then(|| (rect, self.opacity()))
    }
    pub(in crate::desktop) fn exit_idle(&self) -> bool {
        self.persistence_idle()
            && !self.state.snapshot_requested.get()
            && !self.state.link_requested.get()
            && !self.state.unlink_requested.get()
            && self.state.groups.borrow().request.is_none()
    }
    pub(in crate::desktop) fn persistence_idle(&self) -> bool {
        self.is_alive()
            && self.state.drag.get().is_none()
            && !self.state.drag_starting.get()
            && !self.state.menu_open.get()
            && !self.state.freeze_requested.get()
            && !self.state.recycle_requested.get()
            && !self.state.art_requested.get()
            && self.state.pin.request.get().is_none()
    }
    pub(in crate::desktop) fn take_publication_request(&self) -> Option<Request> {
        self.state.publication.request.take()
    }
    pub(in crate::desktop) fn publication_phase(&self, phase: Phase) {
        if self.state.publication.phase.replace(phase) == phase || !self.is_alive() {
            return;
        }
        let busy = matches!(
            phase,
            Phase::Starting | Phase::Publishing | Phase::Recovering | Phase::Stopping
        );
        let title = windows::core::HSTRING::from(match phase {
            Phase::Idle => crate::ui_text::text("integration-publish-live"),
            Phase::Starting => crate::ui_text::text("integration-live-publishing"),
            Phase::Publishing => crate::ui_text::text("integration-live-published"),
            Phase::Recovering => crate::ui_text::text("integration-live-reconnecting"),
            Phase::Stopping => crate::ui_text::text("integration-live-publication-stopping"),
            Phase::Failed => crate::ui_text::text("integration-publication-failed"),
            Phase::Unconfirmed => crate::ui_text::text("integration-publication-unconfirmed"),
        });
        // Hold no RefCell borrow across synchronous Win32 dispatch; modify only this owner's system menu.
        unsafe {
            let menu = GetSystemMenu(self.hwnd, false);
            let allow = self.state.input.borrow().source_input_allowed();
            let _ = EnableMenuItem(
                menu,
                PUBLISH_CONTROL as u32,
                MF_BYCOMMAND
                    | if !busy && allow {
                        MF_ENABLED
                    } else {
                        MF_GRAYED
                    },
            );
            let _ = EnableMenuItem(
                menu,
                PUBLISH_OBSERVE as u32,
                MF_BYCOMMAND
                    | if !busy && allow {
                        MF_ENABLED
                    } else {
                        MF_GRAYED
                    },
            );
            let _ = EnableMenuItem(
                menu,
                PUBLISH_DIRECT as u32,
                MF_BYCOMMAND | if busy { MF_GRAYED } else { MF_ENABLED },
            );
            let _ = ModifyMenuW(
                menu,
                PUBLISH as u32,
                MF_BYCOMMAND | MF_STRING | if busy { MF_GRAYED } else { MF_ENABLED },
                PUBLISH,
                &title,
            );
            let _ = EnableMenuItem(
                menu,
                STOP as u32,
                MF_BYCOMMAND
                    | if busy && phase != Phase::Stopping {
                        MF_ENABLED
                    } else {
                        MF_GRAYED
                    },
            );
        }
    }
}

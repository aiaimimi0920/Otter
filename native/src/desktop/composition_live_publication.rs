//! Live系统菜单只排有界发布/停止意图；网络与帧读回留在Application后台owner。
use super::*;
pub(in crate::desktop) const PUBLISH: usize = 0x2fb0;
pub(in crate::desktop) const STOP: usize = 0x2fc0;
pub(in crate::desktop) const PUBLISH_CONTROL: usize = 0x2fa0;
pub(in crate::desktop) const PUBLISH_OBSERVE: usize = 0x2f90;
#[derive(Clone, Copy)]
pub(in crate::desktop) enum Request {
    Publish,
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
    AppendMenuW(menu, MF_STRING, PUBLISH, w!("发布 Live 到 Loom"))?;
    AppendMenuW(
        menu,
        MF_STRING | MF_GRAYED,
        PUBLISH_CONTROL,
        w!("发布 Live 并允许远程控制"),
    )?;
    AppendMenuW(menu, MF_STRING, STOP, w!("停止 Live 发布"))?;
    AppendMenuW(
        menu,
        MF_STRING | MF_GRAYED,
        PUBLISH_OBSERVE,
        w!("发布 Live 并共享窗口观察"),
    )?;
    let _ = EnableMenuItem(menu, STOP as u32, MF_BYCOMMAND | MF_GRAYED);
    Ok(())
}
pub(super) fn queue(state: &State, command: usize) {
    let phase = state.publication.phase.get();
    if matches!(command, PUBLISH | PUBLISH_CONTROL | PUBLISH_OBSERVE)
        && matches!(phase, Phase::Idle | Phase::Failed | Phase::Unconfirmed)
    {
        state
            .publication
            .request
            .set(Some(if command == PUBLISH_CONTROL {
                Request::PublishInteractive
            } else if command == PUBLISH_OBSERVE {
                Request::PublishObservations
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
        let title = match phase {
            Phase::Idle => w!("发布 Live 到 Loom"),
            Phase::Starting => w!("Live 正在发布…"),
            Phase::Publishing => w!("Live 已发布"),
            Phase::Recovering => w!("Live 重连中…"),
            Phase::Stopping => w!("Live 正在停止发布…"),
            Phase::Failed => w!("发布失败（可重试）"),
            Phase::Unconfirmed => w!("发布已停，服务器关闭未确认"),
        };
        // 不持RefCell借用跨Win32同步派发；仅改本owner的系统菜单。
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
            let _ = ModifyMenuW(
                menu,
                PUBLISH as u32,
                MF_BYCOMMAND | MF_STRING | if busy { MF_GRAYED } else { MF_ENABLED },
                PUBLISH,
                title,
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

//! 宿主的单次输入发布；所有跨进程字段为标量，COMMIT 最后写入，撤销不等待源返回。
use super::*;
use crate::live_hover_owner::{Control, Identity};
use otter_live_hover_hook::native_protocol as protocol;

pub(super) struct Publication {
    owner: Identity,
    nonce: usize,
    popup_scope: otter_live_hover_hook::native_input_scope::Snapshot,
}
impl Publication {
    pub(super) fn publish(
        control: Control,
        target: &Destination,
        packet: Message,
        nonce: usize,
        popup_scope: otter_live_hover_hook::native_input_scope::Snapshot,
    ) -> Result<Self> {
        let owner = control.identity();
        let epoch = control.epoch();
        if !owner.valid() || !protocol::valid_packet(packet.id, packet.wp.0, packet.lp.0) {
            return Err(Error::SourceTransition);
        }
        if !control.valid() {
            return Err(Error::ControlRevoked);
        }
        if !popup_scope.valid_for(target.root().hwnd(), owner.window()) {
            return Err(Error::SourceTransition);
        }
        let publication = Self {
            owner,
            nonce,
            popup_scope,
        };
        unsafe {
            for (key, value) in [
                (protocol::COMMIT, 0),
                (protocol::COOKIE, owner.cookie),
                (protocol::REQUEST_EPOCH, epoch),
                (protocol::ROOT, target.root().hwnd().0 as usize),
                (protocol::DESTINATION, target.hwnd().0 as usize),
                (protocol::MESSAGE, packet.id as usize),
                (protocol::WP, packet.wp.0),
                (protocol::LP, packet.lp.0 as usize),
                (protocol::CLAIMED, 0),
                (protocol::FINISHED, 0),
            ] {
                SetPropW(owner.window(), key, Some(HANDLE(value as *mut _)))
                    .map_err(|_| Error::DeliveryFailed)?;
            }
            popup_scope
                .publish(owner.window())
                .map_err(|_| Error::DeliveryFailed)?;
            if !owner.valid() {
                return Err(Error::SourceTransition);
            }
            if !control.valid() {
                return Err(Error::ControlRevoked);
            }
            SetPropW(
                owner.window(),
                protocol::COMMIT,
                Some(HANDLE(nonce as *mut _)),
            )
            .map_err(|_| Error::DeliveryFailed)?;
        }
        Ok(publication)
    }
    pub(super) fn message(&self) -> Result<Message> {
        let id = otter_live_hover_hook::native_dispatch::message_id();
        if id == 0 {
            return Err(Error::DeliveryFailed);
        }
        Ok(Message {
            id,
            wp: WPARAM(self.owner.hwnd),
            lp: LPARAM(self.nonce as isize),
            moving: false,
            releasing: false,
        })
    }
    pub(super) fn finished(&self) -> bool {
        self.owner.valid()
            && protocol::value(self.owner.window(), protocol::COMMIT) == self.nonce
            && protocol::value(self.owner.window(), protocol::CLAIMED) == self.nonce
            && protocol::value(self.owner.window(), protocol::FINISHED) == self.nonce
    }
}
impl Drop for Publication {
    fn drop(&mut self) {
        if self.owner.valid()
            && protocol::value(self.owner.window(), protocol::COMMIT) == self.nonce
        {
            unsafe {
                // 先撤销提交标志；迟到源回调不能给新事务写完成或重新建立租约。
                for key in protocol::REQUEST_KEYS
                    .into_iter()
                    .chain(self.popup_scope.keys())
                {
                    let _ = RemovePropW(self.owner.window(), key);
                }
            }
        }
    }
}
#[cfg(test)]
#[path = "live_input_native_completion_tests.rs"]
mod completion_tests;
#[cfg(test)]
#[path = "live_input_native_epoch_tests.rs"]
mod epoch_tests;
#[cfg(test)]
#[path = "live_input_native_scope_tests.rs"]
mod scope_tests;

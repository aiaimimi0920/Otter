//! 先保存媒体终态再取消控制 I/O；避免立即唤醒后只看到 Stopped 而丢失撤权原因。
use super::Exit;
use crate::{device_authorization_lease::Lease, live_relay_viewer_runtime::Exit as MediaExit};
use std::sync::{Arc, Mutex};
pub(super) struct Completion {
    lease: Arc<Lease>,
    result: Mutex<Option<Exit>>,
}
impl Completion {
    pub(super) fn new(lease: Arc<Lease>) -> Arc<Self> {
        Arc::new(Self {
            lease,
            result: Mutex::new(None),
        })
    }
    pub(super) fn finish(&self, exit: MediaExit) {
        let result = if self.lease.is_revoked() || exit == MediaExit::Stopped {
            Exit::Stopped
        } else {
            Exit::Media(exit)
        };
        let mut slot = self
            .result
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        if slot.is_none() {
            *slot = Some(result);
        }
        drop(slot);
        self.lease.revoke();
    }
    pub(super) fn exit(&self) -> Option<Exit> {
        *self
            .result
            .lock()
            .unwrap_or_else(|error| error.into_inner())
    }
}

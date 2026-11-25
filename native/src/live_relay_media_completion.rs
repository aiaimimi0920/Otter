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
        let result = match exit {
            MediaExit::DeviceRevoked => Exit::Media(exit),
            _ if self.lease.is_revoked() || exit == MediaExit::Stopped => Exit::Stopped,
            _ => Exit::Media(exit),
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
    // 迟到的普通媒体终态不覆盖已经选定的控制结果。
    pub(super) fn device_revocation_or(&self, fallback: Exit) -> Exit {
        match self.exit() {
            Some(exit @ Exit::Media(MediaExit::DeviceRevoked)) => exit,
            _ => fallback,
        }
    }
    pub(super) fn exit(&self) -> Option<Exit> {
        *self
            .result
            .lock()
            .unwrap_or_else(|error| error.into_inner())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{device_authorization_lease::Grant, device_session_client::tests::session};

    #[test]
    fn revoked_lease_retains_only_explicit_media_revocation() {
        let origin = "https://localhost:1234";
        let mut grant = Grant::new(session(origin, u64::MAX));
        let revoked = Exit::Media(MediaExit::DeviceRevoked);
        for (media, expected) in [
            (MediaExit::DeviceRevoked, revoked),
            (MediaExit::Closed, Exit::Stopped),
            (MediaExit::Stopped, Exit::Stopped),
        ] {
            let lease = Arc::new(grant.lease(origin, 0).unwrap());
            let completion = Completion::new(lease.clone());
            lease.revoke();
            assert_eq!(completion.device_revocation_or(Exit::Closed), Exit::Closed);
            completion.finish(media);
            assert_eq!(completion.exit(), Some(expected));
            for fallback in [Exit::Stopped, Exit::Closed, Exit::Receiver] {
                let expected = if media == MediaExit::DeviceRevoked {
                    revoked
                } else {
                    fallback
                };
                assert_eq!(completion.device_revocation_or(fallback), expected);
            }
        }
    }
}

//! 单次后台入口获取/求值；完成但未消费的结果仍占共享槽，移交只接受同一 Grant。
use super::prepare::{prepare, Connection};
use super::*;
use crate::{
    device_authorization_config::Config, device_session_client::Client,
    live_relay_control_runtime::Slot,
};
use std::{os::windows::io::AsHandle, thread::JoinHandle};
pub struct Ready {
    mounted: Mounted,
    _slot: Slot,
}
impl Ready {
    #[cfg(test)]
    pub(crate) fn probe(&self) -> crate::bounded_worker_process::Probe {
        self.mounted.probe()
    }
    pub fn transfer(
        mut self,
        destination: &Lease,
        current: &MountedSurface,
    ) -> Result<Mounted, Error> {
        if !self.mounted.lease.same_grant(destination) {
            return Err(Error::Stopped);
        }
        destination
            .validate(current.binding.origin(), now()?)
            .map_err(|_| Error::Stopped)?;
        self.mounted.refresh(current)?;
        Ok(self.mounted)
    }
}
pub struct Job {
    lease: Arc<Lease>,
    thread: Option<JoinHandle<Result<Mounted, Error>>>,
    slot: Option<Slot>,
}
impl Job {
    pub fn start(config: Config, lease: Lease, current: MountedSurface) -> io::Result<Self> {
        Self::spawn(Connection::Configured(config), lease, current)
    }
    pub fn start_prepared(
        client: Client,
        lease: Lease,
        current: MountedSurface,
    ) -> io::Result<Self> {
        Self::spawn(Connection::Prepared(client), lease, current)
    }
    fn spawn(connection: Connection, lease: Lease, current: MountedSurface) -> io::Result<Self> {
        let slot = Slot::acquire()?;
        let lease = Arc::new(lease);
        let task = lease.clone();
        let thread = std::thread::Builder::new()
            .name("otter-surface-javascript-mount".into())
            .spawn(move || {
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    prepare(connection, task, &current, None)
                }))
                .unwrap_or(Err(Error::Panicked))
            })?;
        Ok(Self {
            lease,
            thread: Some(thread),
            slot: Some(slot),
        })
    }
    pub fn request_stop(&self) {
        self.lease.revoke();
    }
    pub fn wait_handle(&self) -> Option<BorrowedHandle<'_>> {
        self.thread.as_ref().map(|thread| thread.as_handle())
    }
    pub fn try_join(&mut self) -> Option<Result<Ready, Error>> {
        if !self.thread.as_ref()?.is_finished() {
            return None;
        }
        let result = self.thread.take()?.join().unwrap_or(Err(Error::Panicked));
        if self.lease.is_revoked() {
            self.slot.take();
            return Some(Err(Error::Stopped));
        }
        Some(match result {
            Ok(mounted) => Ok(Ready {
                mounted,
                _slot: self.slot.take().expect("live job slot"),
            }),
            Err(error) => {
                self.slot.take();
                Err(error)
            }
        })
    }
}
impl Drop for Job {
    fn drop(&mut self) {
        // Ready/Mounted 接管时仍共享原请求租约；成功 join 后 Drop 不得撤销已移交会话。
        if let Some(thread) = self.thread.take() {
            self.request_stop();
            let _ = thread.join();
        }
    }
}

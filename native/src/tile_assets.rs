//! One image/media owner and lazy serial Art worker per output, at most one in-flight/one pending; drain old generations before releasing capacity.
use crate::{
    device_authorization_lease::Lease,
    device_session_client::Client,
    tile_shared::{Key, Shared},
    tile_wire::{Layout, Size},
};
#[path = "tile_assets_live.rs"]
pub(crate) mod live;
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc,
    },
    thread::JoinHandle,
};
#[derive(Clone)]
pub(crate) struct Job {
    pub generation: u64,
    pub key: Key,
    pub layout: Layout,
    pub size: Size,
}
pub(crate) struct Assets {
    sender: Option<mpsc::SyncSender<Job>>,
    thread: Option<JoinHandle<()>>,
    lease: Arc<Lease>,
    art: Arc<Lease>,
    busy: Arc<AtomicBool>,
}
impl Assets {
    pub fn start(
        client: Client,
        lease: Arc<Lease>,
        art: Arc<Lease>,
        shared: Arc<Shared>,
    ) -> std::io::Result<Self> {
        use windows::Win32::System::Com::*;
        Self::start_with_com(client, lease, art, shared, || unsafe {
            CoInitializeEx(None, COINIT_MULTITHREADED)
        })
    }
    fn start_with_com(
        client: Client,
        lease: Arc<Lease>,
        art: Arc<Lease>,
        shared: Arc<Shared>,
        initialize: impl FnOnce() -> windows::core::HRESULT + Send + 'static,
    ) -> std::io::Result<Self> {
        let (sender, receiver) = mpsc::sync_channel::<Job>(1);
        let (ready, startup) = mpsc::sync_channel(1);
        let token = lease.clone();
        let art_token = art.clone();
        let busy = Arc::new(AtomicBool::new(false));
        let active = busy.clone();
        let interrupts = crate::tile_live_socket::Interrupts::default();
        let revoked = interrupts.clone();
        let ended = shared.clone();
        let thread = std::thread::Builder::new()
            .name("otter-tile-images".into())
            .spawn(move || {
                use windows::Win32::System::Com::*;
                if let Err(error) = initialize().ok() {
                    let _ = ready.send(Err(error));
                    return;
                }
                struct Com;
                impl Drop for Com {
                    fn drop(&mut self) {
                        unsafe {
                            CoUninitialize();
                        }
                    }
                }
                let _com = Com;
                if ready.send(Ok(())).is_err() {
                    return;
                }
                let mut cache = BTreeMap::new();
                while let Ok(job) = receiver.recv() {
                    if shared.lock().stopped {
                        break;
                    }
                    let result = live::run(
                        &client,
                        &token,
                        &art_token,
                        &shared,
                        &job,
                        &mut cache,
                        &interrupts,
                    );
                    if let Err(reason) = result {
                        shared.unavailable(job.generation, &job.key, reason);
                    }
                    active.store(false, Ordering::Release);
                    if result.is_err() && !shared.wait(std::time::Duration::from_secs(2)) {
                        break;
                    }
                }
            })?;
        // Only the background presenter waits for COM initialization; reap failed threads first without starting Wall network requests.
        if let Err(error) = startup
            .recv()
            .map_err(std::io::Error::other)
            .and_then(|result| result.map_err(std::io::Error::other))
        {
            let _ = thread.join();
            return Err(error);
        }
        lease.notify_revocation(Arc::new(move || {
            ended.stop();
            for handle in revoked.lock().unwrap_or_else(|e| e.into_inner()).drain(..) {
                let _ = handle.cancel();
            }
        }));
        Ok(Self {
            sender: Some(sender),
            thread: Some(thread),
            lease,
            art,
            busy,
        })
    }
    pub fn submit(&self, job: impl FnOnce() -> Job) -> bool {
        if self.busy.swap(true, Ordering::AcqRel) {
            return false;
        }
        let sent = self
            .sender
            .as_ref()
            .is_some_and(|s| s.try_send(job()).is_ok());
        if !sent {
            self.busy.store(false, Ordering::Release);
        }
        sent
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::device_session_client::tests::{fixture_with_pem, session};

    #[test]
    fn tile_assets_reports_com_failure_before_accepting_jobs() {
        let (_, client, _, _) = fixture_with_pem();
        let origin = client.origin();
        let mut grant = crate::device_authorization_lease::Grant::new(session(origin, u64::MAX));
        let images = Arc::new(grant.lease(origin, 0).unwrap());
        let art = Arc::new(grant.lease(origin, 0).unwrap());
        let Err(error) =
            Assets::start_with_com(client, images, art, Arc::new(Shared::default()), || {
                windows::Win32::Foundation::E_FAIL
            })
        else {
            panic!("failed COM initialization must not publish an asset worker");
        };
        let cause = error
            .get_ref()
            .and_then(|e| e.downcast_ref::<windows::core::Error>())
            .unwrap();
        assert_eq!(cause.code(), windows::Win32::Foundation::E_FAIL);
    }

    #[test]
    fn tile_assets_busy_rejection_does_not_construct_job() {
        let (_, client, _, _) = fixture_with_pem();
        let origin = client.origin();
        let mut grant = crate::device_authorization_lease::Grant::new(session(origin, u64::MAX));
        let images = Arc::new(grant.lease(origin, 0).unwrap());
        let art = Arc::new(grant.lease(origin, 0).unwrap());
        let assets = Assets::start(client, images, art, Arc::new(Shared::default())).unwrap();
        assets.busy.store(true, Ordering::Release);
        assert!(!assets.submit(|| panic!("busy worker must not construct a job")));
    }
}
impl Drop for Assets {
    fn drop(&mut self) {
        self.lease.revoke();
        self.art.revoke();
        self.sender.take();
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

//! 每输出一个图片/媒体owner和惰性Art串行worker，至多一在途/一待办；旧代际排空才释放容量。
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
        let (sender, receiver) = mpsc::sync_channel::<Job>(1);
        let token = lease.clone();
        let art_token = art.clone();
        let busy = Arc::new(AtomicBool::new(false));
        let active = busy.clone();
        let interrupts = crate::tile_live_socket::Interrupts::default();
        let revoked = interrupts.clone();
        let ended = shared.clone();
        lease.notify_revocation(Arc::new(move || {
            ended.stop();
            for handle in revoked.lock().unwrap_or_else(|e| e.into_inner()).drain(..) {
                let _ = handle.cancel();
            }
        }));
        let thread = std::thread::Builder::new()
            .name("otter-tile-images".into())
            .spawn(move || {
                use windows::Win32::System::Com::*;
                if unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) }.is_err() {
                    shared.lock().status = "图片线程初始化失败";
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
        Ok(Self {
            sender: Some(sender),
            thread: Some(thread),
            lease,
            art,
            busy,
        })
    }
    pub fn submit(&self, job: Job) -> bool {
        if self.busy.swap(true, Ordering::AcqRel) {
            return false;
        }
        let sent = self
            .sender
            .as_ref()
            .is_some_and(|s| s.try_send(job).is_ok());
        if !sent {
            self.busy.store(false, Ordering::Release);
        }
        sent
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

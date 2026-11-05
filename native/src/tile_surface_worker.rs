//! 每个在途Tile job最多一个Art worker/单槽快照；慢HTTP不阻塞原Live轮询，不并行复用Lease。
use crate::{
    device_authorization_lease::Lease,
    device_session_client::Client,
    tile_assets::{
        live::{phase, Phase},
        Job,
    },
    tile_shared::Shared,
    tile_surface_owner::{Art, Views},
};
use std::{
    collections::BTreeMap,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread::JoinHandle,
    time::Duration,
};
pub(crate) struct Prepared {
    pub frames: BTreeMap<String, Arc<Art>>,
    pub pixels: u64,
}
impl Prepared {
    fn same(&self, other: &Self) -> bool {
        self.pixels == other.pixels
            && self.frames.len() == other.frames.len()
            && self
                .frames
                .iter()
                .all(|(id, art)| other.frames.get(id).is_some_and(|a| Arc::ptr_eq(a, art)))
    }
}
pub(crate) struct Worker {
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<Result<(), &'static str>>>,
}
struct Retire<'a> {
    shared: &'a Shared,
    job: &'a Job,
}
impl Drop for Retire<'_> {
    fn drop(&mut self) {
        let mut s = self.shared.lock();
        if s.generation == self.job.generation && s.desired.as_ref() == Some(&self.job.key) {
            s.art = None;
            s.actions.clear();
            s.unavailable(self.job.generation, &self.job.key, "Art准备已结束");
        }
    }
}
impl Worker {
    pub fn start(
        client: &Client,
        lease: &Arc<Lease>,
        shared: &Arc<Shared>,
        job: &Job,
        ids: Vec<String>,
        other_pixels: u64,
    ) -> Result<Self, &'static str> {
        let stop = Arc::new(AtomicBool::new(false));
        if ids.is_empty() {
            return Ok(Self { stop, thread: None });
        }
        {
            let mut s = shared.lock();
            if phase(&s, job) == Phase::Active {
                s.art = None;
                s.actions.clear();
                s.unavailable(job.generation, &job.key, "正在准备Art内容");
            }
        }
        let (client, lease, shared, job, ended) = (
            client.clone(),
            lease.clone(),
            shared.clone(),
            job.clone(),
            stop.clone(),
        );
        let thread = std::thread::Builder::new()
            .name("otter-tile-art".into())
            .spawn(move || {
                use windows::Win32::System::Com::*;
                unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) }
                    .ok()
                    .map_err(|_| "Art线程初始化失败")?;
                struct Com;
                impl Drop for Com {
                    fn drop(&mut self) {
                        unsafe { CoUninitialize() };
                    }
                }
                let _com = Com;
                let mut views = Views::new(&client, &lease);
                // 先撤掉单槽发布资格，再做Views的有界网络清理，不能在close期间复活旧准备。
                let _retire = Retire {
                    shared: &shared,
                    job: &job,
                };
                while !ended.load(Ordering::Acquire) && !lease.is_revoked() {
                    // 不让match临时MutexGuard活到Held分支内；wait会重新获取同一把锁。
                    let mode = phase(&shared.lock(), &job);
                    match mode {
                        Phase::Ended => break,
                        Phase::Held => {
                            views.poll_cancel(&shared, &job);
                            shared.wait(Duration::from_millis(16));
                            continue;
                        }
                        Phase::Active => {}
                    }
                    let result = views.poll(&ids, &shared, &job, other_pixels);
                    let mut s = shared.lock();
                    // 错误与快照都服从同一代际；冻结中的迟到state不改实际paint保留帧。
                    match phase(&s, &job) {
                        Phase::Ended => break,
                        Phase::Held => continue,
                        Phase::Active => {
                            result?;
                        }
                    }
                    if ended.load(Ordering::Acquire) {
                        break;
                    }
                    let ready = Prepared {
                        frames: views.frames(),
                        pixels: views.pixels(),
                    };
                    if s.art.as_ref().is_none_or(|old| !old.same(&ready)) {
                        s.art = Some(Arc::new(ready));
                    }
                    drop(s);
                    shared.wait(Duration::from_millis(16));
                }
                // Views先关闭可信view，再退出COM；取消/撤权沿原Lease，不借新凭据做清理。
                Ok(())
            })
            .map_err(|_| "Art线程启动失败")?;
        Ok(Self {
            stop,
            thread: Some(thread),
        })
    }
    pub fn check(&mut self) -> Result<(), &'static str> {
        if self.thread.as_ref().is_some_and(|t| t.is_finished()) {
            return match self.thread.take().unwrap().join() {
                Ok(Err(reason)) => Err(reason),
                _ => Err("Art线程已结束"),
            };
        }
        Ok(())
    }
}
impl Drop for Worker {
    fn drop(&mut self) {
        // 无待办队列；旧worker实际退出前不释放job容量。正常换代最多等原HTTP期限，终止由Lease取消唤醒。
        self.stop.store(true, Ordering::Release);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
#[cfg(test)]
#[path = "tile_surface_worker_tests.rs"]
mod tests;

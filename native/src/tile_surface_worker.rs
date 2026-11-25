//! At most one Art worker/single-slot snapshot per in-flight Tile job; slow HTTP does not block original Live polling or reuse Leases concurrently.
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
            s.unavailable(
                self.job.generation,
                &self.job.key,
                "tile-art-preparation-ended",
            );
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
                s.unavailable(job.generation, &job.key, "tile-art-preparing");
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
                    .map_err(|_| "tile-art-thread-init-failed")?;
                struct Com;
                impl Drop for Com {
                    fn drop(&mut self) {
                        unsafe { CoUninitialize() };
                    }
                }
                let _com = Com;
                let mut views = Views::new(&client, &lease);
                // Revoke single-slot publication eligibility before bounded Views network cleanup; old preparation cannot revive during close.
                let _retire = Retire {
                    shared: &shared,
                    job: &job,
                };
                while !ended.load(Ordering::Acquire) && !lease.is_revoked() {
                    // Do not let match's temporary MutexGuard survive into the Held branch; wait reacquires the same lock.
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
                    // Errors and snapshots obey the same generation; late state while frozen never changes the retained actually painted frame.
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
                // Views closes trusted views before exiting COM; cancellation/revocation uses the original Lease without new cleanup credentials.
                Ok(())
            })
            .map_err(|_| "tile-art-thread-start-failed")?;
        Ok(Self {
            stop,
            thread: Some(thread),
        })
    }
    pub fn check(&mut self) -> Result<(), &'static str> {
        if self.thread.as_ref().is_some_and(|t| t.is_finished()) {
            return match self.thread.take().unwrap().join() {
                Ok(Err(reason)) => Err(reason),
                _ => Err("tile-art-thread-ended"),
            };
        }
        Ok(())
    }
}
impl Drop for Worker {
    fn drop(&mut self) {
        // No pending queue; retain job capacity until old workers actually exit. Normal generation changes wait at most the original HTTP deadline, and Lease cancellation wakes termination.
        self.stop.store(true, Ordering::Release);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
#[cfg(test)]
#[path = "tile_surface_worker_tests.rs"]
mod tests;

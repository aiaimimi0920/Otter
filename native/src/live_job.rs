//! Live background owner: reuse CaptureJob thread reclamation, close frame sources to wake WGC, and bound simultaneous session tasks.
pub use crate::live_window_target::MAX_SESSIONS;
use crate::{
    capture_job::CaptureJob,
    live_frames::{Consumer, Source},
    live_texture::Texture,
};
use std::{
    io,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc, LazyLock,
    },
};
use windows::Win32::Graphics::Gdi::HMONITOR;
static ACTIVE: LazyLock<Arc<AtomicUsize>> = LazyLock::new(|| Arc::new(AtomicUsize::new(0)));
fn require_owner_apartment() -> io::Result<()> {
    use windows::Win32::System::Com::*;
    let mut apartment = APTTYPE_STA;
    let mut qualifier = APTTYPEQUALIFIER_NONE;
    unsafe { CoGetApartmentType(&mut apartment, &mut qualifier) }.map_err(|error| {
        io::Error::other(format!(
            "Live owner requires a persistent COM apartment: {error}"
        ))
    })?;
    if qualifier == APTTYPEQUALIFIER_IMPLICIT_MTA {
        return Err(io::Error::other(
            "Live owner cannot depend on a transient implicit MTA",
        ));
    }
    Ok(())
}
struct Slot(Arc<AtomicUsize>);
impl Slot {
    fn acquire(active: &Arc<AtomicUsize>) -> io::Result<Self> {
        active
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |count| {
                (count < MAX_SESSIONS).then_some(count + 1)
            })
            .map_err(|_| io::Error::new(io::ErrorKind::WouldBlock, "Live session limit reached"))?;
        Ok(Self(active.clone()))
    }
}
impl Drop for Slot {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::AcqRel);
    }
}
struct CloseSource<T>(Arc<Source<T>>);
impl<T> Drop for CloseSource<T> {
    fn drop(&mut self) {
        self.0.close();
    }
}
pub struct LiveJob<T> {
    // Destroy WinEvent hooks on their installation thread; the window-task owner remains on the message-pumping UI thread.
    watcher: Option<crate::live_window_target::Watcher>,
    source: Option<crate::live_window_target::Target>,
    source_stopped: bool,
    worker: CaptureJob<()>,
    frames: Arc<Source<T>>,
}
impl<T: Send + Sync + 'static> LiveJob<T> {
    fn spawn(
        active: &Arc<AtomicUsize>,
        run: impl FnOnce(&Arc<Source<T>>, &AtomicBool) -> Result<(), String> + Send + 'static,
    ) -> io::Result<(Self, Consumer<T>)> {
        Self::spawn_bound(active, Ok, run)
    }
    pub fn spawn_bound<U>(
        active: &Arc<AtomicUsize>,
        bind: impl FnOnce(Consumer<T>) -> io::Result<U>,
        run: impl FnOnce(&Arc<Source<T>>, &AtomicBool) -> Result<(), String> + Send + 'static,
    ) -> io::Result<(Self, U)> {
        let slot = Slot::acquire(active)?;
        let frames = Arc::new(Source::new());
        let consumer = frames
            .subscribe()
            .map_err(|error| io::Error::other(format!("{error:?}")))?;
        // Create both guards outside the closure so failed thread creation or cancellation before invocation still reclaims them.
        let close = CloseSource(frames.clone());
        // UI binding, including capture exclusion, must succeed before creating the capture thread.
        let bound = bind(consumer)?;
        let worker = CaptureJob::spawn(move |cancel| {
            let _slot = slot;
            let result = run(&close.0, cancel);
            drop(close);
            result
        })?;
        Ok((
            Self {
                watcher: None,
                source: None,
                source_stopped: false,
                worker,
                frames,
            },
            bound,
        ))
    }
}
impl LiveJob<Texture> {
    /// Bind Target at user selection; pass numeric identity/original process owner across threads, without reading window titles.
    /// The calling thread must keep pumping window messages; LiveJob stays there to receive/unregister destruction-event listeners.
    pub fn start_window_bound<U>(
        target: crate::live_window_target::Target,
        bind: impl FnOnce(Consumer<Texture>) -> io::Result<U>,
    ) -> io::Result<(Self, U)> {
        Self::start_window_region_bound(target, None, bind)
    }
    pub fn start_window_region_bound<U>(
        target: crate::live_window_target::Target,
        crop: Option<crate::live_window_target::region::Crop>,
        bind: impl FnOnce(Consumer<Texture>) -> io::Result<U>,
    ) -> io::Result<(Self, U)> {
        target.validate().map_err(io::Error::other)?;
        if let Some(crop) = crop {
            crop.screen(target.frame_bounds().map_err(io::Error::other)?)
                .map_err(io::Error::other)?;
        }
        require_owner_apartment()?;
        let worker_target = target.clone();
        let (mut job, bound) = Self::spawn_bound(&ACTIVE, bind, move |frames, cancel| {
            if cancel.load(Ordering::Acquire) || !frames.has_consumers() {
                return Ok(());
            }
            let result = run_session(
                crate::capture_wgc::live::Session::window_region(
                    &worker_target,
                    crop,
                    frames.clone(),
                ),
                cancel,
                || worker_target.validate().is_ok(),
            );
            if result.is_ok()
                && !cancel.load(Ordering::Acquire)
                && worker_target.validate().is_err()
            {
                Err("Live capture failed: TargetChanged".into())
            } else {
                result
            }
        })?;
        let frames = job.frames.clone();
        job.watcher = Some(
            target
                .watch(Arc::new(move || frames.close()))
                .map_err(io::Error::other)?,
        );
        job.source = Some(target);
        Ok((job, bound))
    }
    /// Explicitly authorized monitor capture; the host must first select the target and configure capture exclusion, never call automatically at app startup.
    pub fn start_monitor(monitor: HMONITOR) -> io::Result<(Self, Consumer<Texture>)> {
        if monitor.0.is_null() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Invalid Live monitor",
            ));
        }
        let monitor = monitor.0 as usize;
        require_owner_apartment()?;
        Self::spawn(&ACTIVE, move |frames, cancel| {
            use crate::capture_wgc::live::Session;
            if cancel.load(Ordering::Acquire) || !frames.has_consumers() {
                return Ok(());
            }
            run_session(
                Session::monitor(HMONITOR(monitor as *mut _), frames.clone()),
                cancel,
                || true,
            )
        })
    }
    /// Physical screen region; background work binds one monitor and rechecks layout before/after delivery, without implicitly clipping cross-monitor regions.
    pub fn start_region(
        screen: windows::Win32::Foundation::RECT,
    ) -> io::Result<(Self, Consumer<Texture>)> {
        Self::start_region_bound(screen, Ok)
    }
    /// Bind synchronously on the calling thread; failure starts no capture, and the worker starts only after success.
    pub fn start_region_bound<U>(
        screen: windows::Win32::Foundation::RECT,
        bind: impl FnOnce(Consumer<Texture>) -> io::Result<U>,
    ) -> io::Result<(Self, U)> {
        crate::capture_target::dimensions(screen)
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "Invalid Live region"))?;
        require_owner_apartment()?;
        Self::spawn_bound(&ACTIVE, bind, move |frames, cancel| {
            if cancel.load(Ordering::Acquire) || !frames.has_consumers() {
                return Ok(());
            }
            let target = crate::capture_target::Target::resolve(screen)
                .map_err(|error| format!("Live target failed: {error:?}"))?;
            run_session(
                crate::capture_wgc::live::Session::monitor_region(
                    target.monitor(),
                    target.region(),
                    frames.clone(),
                ),
                cancel,
                || target.validate().is_ok(),
            )
        })
    }
}
fn run_session(
    session: Result<crate::capture_wgc::live::Session, crate::capture_wgc::live::Error>,
    cancel: &AtomicBool,
    mut validate: impl FnMut() -> bool,
) -> Result<(), String> {
    use crate::capture_wgc::live::Error;
    let mut session = match session {
        Ok(session) => session,
        Err(Error::NoConsumers) => return Ok(()),
        Err(error) => return Err(format!("Live startup failed: {error:?}")),
    };
    while !cancel.load(Ordering::Acquire) {
        match session.next_frame_checked(&mut validate) {
            Ok(_) => {}
            Err(Error::Cancelled | Error::NoConsumers) => break,
            Err(error) => return Err(format!("Live capture failed: {error:?}")),
        }
    }
    Ok(())
}
impl<T> LiveJob<T> {
    /// Wait alongside thread handles; same-target clones may share this handle, so deduplicate wait sets.
    pub fn source_process_handle(&self) -> Option<std::os::windows::io::BorrowedHandle<'_>> {
        if self.source_stopped {
            return None;
        }
        self.source
            .as_ref()
            .map(crate::live_window_target::Target::process_handle)
    }
    pub fn refresh_source(&mut self) {
        if !self.source_stopped
            && self
                .source
                .as_ref()
                .is_some_and(|source| source.validate().is_err())
        {
            // Do not set user-cancellation flags; preserve worker source-invalid errors and only wake idle frame waits.
            self.source_stopped = true;
            self.watcher.take();
            self.frames.close();
        }
    }
    pub fn completion_handle(&self) -> Option<std::os::windows::io::BorrowedHandle<'_>> {
        self.worker.completion_handle()
    }
    pub fn subscribe(&self) -> Result<Consumer<T>, crate::live_frames::Error> {
        self.frames.subscribe()
    }
    pub fn cancel(&self) {
        self.worker.cancel();
        // Setting AtomicBool alone cannot wake idle-image event waits; closing the source invokes registered idle callbacks.
        self.frames.close();
    }
    pub fn take_ready(&mut self) -> Option<Result<(), String>> {
        self.refresh_source();
        let result = self.worker.take_ready();
        if result.is_some() {
            self.watcher.take();
            self.source.take();
        }
        result
    }
    pub fn pending(&self) -> bool {
        self.worker.pending()
    }
}
impl<T> Drop for LiveJob<T> {
    fn drop(&mut self) {
        self.watcher.take();
        self.cancel();
        // CaptureJob joins afterward; entered OS/driver startup/shutdown calls may still be uninterruptible.
    }
}
#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-live")))]
#[path = "live_job_tests.rs"]
mod tests;

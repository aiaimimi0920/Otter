//! Live 后台 owner：复用 CaptureJob 回收线程，关闭帧源唤醒 WGC，限制同时存活的会话任务。
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
pub const MAX_SESSIONS: usize = 16;
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
    // WinEvent hook 必须在安装线程销毁；窗口任务 owner 留在泵消息的 UI 线程。
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
    pub(crate) fn spawn_bound<U>(
        active: &Arc<AtomicUsize>,
        bind: impl FnOnce(Consumer<T>) -> io::Result<U>,
        run: impl FnOnce(&Arc<Source<T>>, &AtomicBool) -> Result<(), String> + Send + 'static,
    ) -> io::Result<(Self, U)> {
        let slot = Slot::acquire(active)?;
        let frames = Arc::new(Source::new());
        let consumer = frames
            .subscribe()
            .map_err(|error| io::Error::other(format!("{error:?}")))?;
        // 两个 guard 在闭包外创建：即使线程创建失败或取消使闭包未执行，也会回收。
        let close = CloseSource(frames.clone());
        // UI 绑定（包括窗口捕获排除）先成功，才允许创建采集线程。
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
    /// Target 必须在用户选择时绑定；跨线程传递数字身份和原进程 owner，不读取窗口标题。
    /// 调用线程必须持续泵窗口消息；LiveJob owner 留在该线程以接收并注销销毁事件监听。
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
    /// 显式授权的显示器捕获；宿主需先完成目标选择与捕获排除，不得在启动应用时自动调用。
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
    /// 物理屏幕区域；后台绑定单显示器，交付前后复核布局，跨屏不隐式截断。
    pub fn start_region(
        screen: windows::Win32::Foundation::RECT,
    ) -> io::Result<(Self, Consumer<Texture>)> {
        Self::start_region_bound(screen, Ok)
    }
    /// 绑定在调用线程同步执行；失败不会启动捕获，成功后才启动 worker。
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
    /// 与线程句柄一起等待；同一目标的克隆可能共享此句柄，等待集合必须去重。
    pub fn source_process_handle(&self) -> Option<std::os::windows::io::BorrowedHandle<'_>> {
        if self.source_stopped {
            return None;
        }
        self.source
            .as_ref()
            .map(crate::live_window_target::Target::process_handle)
    }
    pub(crate) fn refresh_source(&mut self) {
        if !self.source_stopped
            && self
                .source
                .as_ref()
                .is_some_and(|source| source.validate().is_err())
        {
            // 不设置用户取消标志，保留 worker 的源失效错误；只唤醒静止的帧等待。
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
        // 只设置 AtomicBool 不足以唤醒静止画面的事件等待；关闭源触发已注册 idle 回调。
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
        // CaptureJob 随后 join；已进入的 OS/驱动启动或关闭调用仍可能不可中断。
    }
}
#[cfg(test)]
#[path = "live_job_tests.rs"]
mod tests;

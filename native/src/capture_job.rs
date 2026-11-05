//! 一次性后台截图 owner；单结果、不可撤销取消、退出 join，不分离线程或建立空闲轮询。
use crate::capture_dynamic::{Mode, Outcome};
use std::{
    io,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread::{self, JoinHandle},
};
use windows::Win32::Foundation::RECT;
#[path = "capture_delivery.rs"]
mod delivery;
pub use delivery::{Captured, PreparedCapture};
#[path = "capture_context.rs"]
mod context;
pub(crate) use context::Context;

pub struct CaptureJob<T = Outcome> {
    cancel: Arc<AtomicBool>,
    worker: Option<JoinHandle<Result<T, String>>>,
}
impl CaptureJob {
    /// 每个对象只启动一次采集。UI owner 仍须限制活动任务数量并复核文档/窗口代次。
    pub fn start(screen: RECT, mode: Mode, standard_region: bool, gain: f32) -> io::Result<Self> {
        Self::spawn(move |cancel| {
            crate::desktop::capture::dynamic_region(screen, mode, standard_region, gain, cancel)
        })
    }
}
impl CaptureJob<PreparedCapture> {
    /// 捕获和缓存编码都在同一 worker；最终文件只在 UI 接受 PreparedCapture 后发布。
    pub fn start_cached(
        screen: RECT,
        mode: Mode,
        standard_region: bool,
        gain: f32,
        profile: std::path::PathBuf,
        stem: String,
        policy: crate::cache_policy::Policy,
    ) -> io::Result<Self> {
        Self::spawn(move |cancel| {
            let outcome = crate::desktop::capture::dynamic_region(
                screen,
                mode,
                standard_region,
                gain,
                cancel,
            )?;
            PreparedCapture::prepare(outcome, &profile, &stem, policy, cancel)
                .map_err(|error| error.to_string())
        })
    }
}
impl<T: Send + 'static> CaptureJob<T> {
    pub(crate) fn spawn(
        work: impl FnOnce(&AtomicBool) -> Result<T, String> + Send + 'static,
    ) -> io::Result<Self> {
        let cancel = Arc::new(AtomicBool::new(false));
        let signal = cancel.clone();
        let worker = thread::Builder::new()
            .name("otter-capture".into())
            .spawn(move || {
                if signal.load(Ordering::Acquire) {
                    return Err("Capture cancelled".into());
                }
                let result = work(&signal);
                // 窗口关闭或用户取消可发生在不可中断驱动调用中；归来时必须丢弃迟到结果。
                if signal.load(Ordering::Acquire) {
                    Err("Capture cancelled".into())
                } else {
                    result
                }
            })?;
        Ok(Self {
            cancel,
            worker: Some(worker),
        })
    }
}
impl<T> CaptureJob<T> {
    /// 借用真实线程句柄供 UI 消息等待使用；必须在 take_ready/Drop 前结束借用。
    pub fn completion_handle(&self) -> Option<std::os::windows::io::BorrowedHandle<'_>> {
        use std::os::windows::io::AsHandle;
        self.worker.as_ref().map(AsHandle::as_handle)
    }
    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::Release);
    }
    pub fn pending(&self) -> bool {
        self.worker.is_some()
    }
    /// 尚未完成时立即返回 None。结果至多提取一次；取消后不会返回成功图像。
    pub fn take_ready(&mut self) -> Option<Result<T, String>> {
        if !self.worker.as_ref()?.is_finished() {
            return None;
        }
        let result = self
            .worker
            .take()?
            .join()
            .unwrap_or_else(|_| Err("Capture worker panicked".into()));
        if self.cancel.load(Ordering::Acquire) {
            Some(Err("Capture cancelled".into()))
        } else {
            Some(result)
        }
    }
}
impl<T> Drop for CaptureJob<T> {
    fn drop(&mut self) {
        self.cancel();
        // 不能用 detach 隐藏泄漏；join 仍可能等待已进入的 OS/驱动调用，UI 需保留关闭状态。
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}
#[cfg(test)]
#[path = "capture_job_tests.rs"]
mod tests;

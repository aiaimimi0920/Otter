//! 有界媒体 worker；COM/codec 仅在线程内存活，停止不排在输入队列末尾。
use crate::{
    live_relay_media_pipeline::{self as pipeline, Pipeline, Progress},
    live_relay_receive::Frame,
    live_relay_viewer_session::{Permit, ViewerSession},
};
use std::{
    io,
    os::windows::io::{AsHandle, BorrowedHandle},
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        mpsc, Arc,
    },
    thread::JoinHandle,
    time::Duration,
};
pub const INPUT_CAPACITY: usize = 2;
const REPLY_CAPACITY: usize = 4;
static ACTIVE: AtomicUsize = AtomicUsize::new(0);
struct Slot;
impl Slot {
    fn acquire() -> io::Result<Self> {
        ACTIVE
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |n| {
                (n < 8).then_some(n + 1)
            })
            .map_err(|_| {
                io::Error::new(io::ErrorKind::WouldBlock, "Live media worker limit reached")
            })?;
        Ok(Self)
    }
}
impl Drop for Slot {
    fn drop(&mut self) {
        ACTIVE.fetch_sub(1, Ordering::AcqRel);
    }
}
enum Command {
    Frame(Frame),
    Wake,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Exit {
    Stopped,
    Pipeline(pipeline::Error),
    RepliesFull,
    Panicked,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SubmitError {
    Full,
    Stopped,
}
struct Retire {
    session: Arc<ViewerSession>,
    permit: Permit,
}
impl Drop for Retire {
    fn drop(&mut self) {
        let _ = self.session.suspend_if_current(&self.permit);
    }
}
pub struct Worker {
    sender: Option<mpsc::SyncSender<Command>>,
    replies: mpsc::Receiver<Progress>,
    stop: Arc<AtomicBool>,
    fallback: Arc<AtomicBool>,
    session: Arc<ViewerSession>,
    permit: Permit,
    thread: Option<JoinHandle<Exit>>,
}
impl Worker {
    pub fn start(session: Arc<ViewerSession>, permit: Permit) -> io::Result<Self> {
        session
            .validate(&permit)
            .map_err(|_| io::Error::new(io::ErrorKind::PermissionDenied, "Media permit invalid"))?;
        let slot = Slot::acquire()?;
        let (sender, commands) = mpsc::sync_channel(INPUT_CAPACITY);
        let (reply, replies) = mpsc::sync_channel(REPLY_CAPACITY);
        let stop = Arc::new(AtomicBool::new(false));
        let fallback = Arc::new(AtomicBool::new(false));
        let fallback_report = fallback.clone();
        let retired = Retire {
            session: session.clone(),
            permit: permit.clone(),
        };
        let cancelled = stop.clone();
        let thread = std::thread::Builder::new()
            .name("otter-live-media".into())
            .spawn(move || {
                let _slot = slot;
                let _retired = retired;
                run(
                    &_retired.session,
                    &_retired.permit,
                    &cancelled,
                    commands,
                    reply,
                    &fallback_report,
                )
            })?;
        Ok(Self {
            sender: Some(sender),
            replies,
            stop,
            fallback,
            session,
            permit,
            thread: Some(thread),
        })
    }
    /// 满载不是静默丢视频帧：撤销此代次并返回 Full，让 transport 执行显式恢复。
    pub fn submit(&self, frame: Frame) -> Result<(), SubmitError> {
        if self.stop.load(Ordering::Acquire) || self.session.validate(&self.permit).is_err() {
            return Err(SubmitError::Stopped);
        }
        let Some(sender) = &self.sender else {
            return Err(SubmitError::Stopped);
        };
        match sender.try_send(Command::Frame(frame)) {
            Ok(()) => Ok(()),
            Err(mpsc::TrySendError::Full(_)) => {
                self.request_stop();
                Err(SubmitError::Full)
            }
            Err(mpsc::TrySendError::Disconnected(_)) => Err(SubmitError::Stopped),
        }
    }
    pub fn request_stop(&self) {
        self.stop.store(true, Ordering::Release);
        let _ = self.session.suspend_if_current(&self.permit);
        if let Some(sender) = &self.sender {
            let _ = sender.try_send(Command::Wake);
        }
    }
    pub fn is_current(&self) -> bool {
        !self.stop.load(Ordering::Acquire) && self.session.validate(&self.permit).is_ok()
    }
    pub(super) fn prefers_legacy(&self) -> bool {
        self.fallback.load(Ordering::Acquire)
    }
    /// 仅由已认证 Device socket 的精确撤销 Close 调用；普通断网使用 request_stop。
    pub fn revoke(&self) {
        let _ = self.session.close_if_current(&self.permit);
        self.request_stop();
    }
    pub fn try_reply(&self) -> Option<Progress> {
        self.replies.try_recv().ok()
    }
    pub fn wait_handle(&self) -> Option<BorrowedHandle<'_>> {
        self.thread.as_ref().map(AsHandle::as_handle)
    }
    pub fn try_join(&mut self) -> Option<Exit> {
        if !self.thread.as_ref()?.is_finished() {
            return None;
        }
        self.sender.take();
        Some(self.thread.take().unwrap().join().unwrap_or(Exit::Panicked))
    }
}
impl Drop for Worker {
    fn drop(&mut self) {
        self.request_stop();
        self.sender.take();
        // 最后兜底避免 detach；UI 正常关闭必须等待句柄并 try_join，而非在回调里 Drop 活跃 worker。
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
fn run(
    session: &Arc<ViewerSession>,
    permit: &Permit,
    stop: &AtomicBool,
    commands: mpsc::Receiver<Command>,
    reply: mpsc::SyncSender<Progress>,
    fallback: &AtomicBool,
) -> Exit {
    let mut pipeline = match Pipeline::new(session.clone(), permit.clone()) {
        Ok(p) => p,
        Err(e) => return Exit::Pipeline(e),
    };
    loop {
        if stop.load(Ordering::Acquire) {
            return Exit::Stopped;
        }
        if let Err(e) = pipeline.check() {
            return Exit::Pipeline(e);
        }
        // recv 是内核等待，不忙轮询；100ms 兜底观察由会话其他 owner 发出的取消。
        match commands.recv_timeout(Duration::from_millis(100)) {
            Ok(Command::Wake) => {}
            Ok(Command::Frame(frame)) => {
                if stop.load(Ordering::Acquire) {
                    return Exit::Stopped;
                }
                match pipeline.receive(frame) {
                    Ok(progress) => {
                        if matches!(progress, Progress::Fallback { .. }) {
                            fallback.store(true, Ordering::Release);
                        }
                        if reply.try_send(progress).is_err() {
                            return Exit::RepliesFull;
                        }
                    }
                    Err(e) => return Exit::Pipeline(e),
                }
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => return Exit::Stopped,
        }
    }
}
#[cfg(test)]
#[path = "live_relay_media_worker_tests.rs"]
mod tests;

//! 单所有者后台会话：至多一个在途命令，禁止 UI 堆积截图；Drop 关闭通道并 join。
use super::*;
#[path = "long_capture_active.rs"]
mod active;
#[path = "long_capture_detect.rs"]
mod detect;
#[path = "long_capture_source.rs"]
mod source;
pub use source::CaptureSource;
use std::{
    sync::{atomic::Ordering, mpsc, Arc},
    thread::JoinHandle,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DispatchError {
    Busy,
    Stopped,
    Unavailable,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Snapshot {
    pub axis: Option<Axis>,
    pub direction: Option<Direction>,
    pub spatial_position: Option<(i64, i64)>,
    pub frame_count: usize,
    pub confirmed_edges: Option<(u32, u32)>,
}
pub enum Completion {
    SpatialAppend(Result<SpatialAppend, Error>),
    SpatialCompose(Result<SpatialComposite, Error>),
    SpatialPreview(Result<SpatialComposite, Error>),
    Append(Result<Append, Error>),
    Compose(Result<Composite, Error>),
    Preview(Result<Composite, Error>),
    WorkerStopped,
    CaptureFailed(String),
}
pub struct Reply {
    pub can_scroll: bool,
    pub can_move: bool,
    pub snapshot: Snapshot,
    pub completion: Completion,
}
enum Command {
    MoveCapture((i32, i32)),
    AppendAt(Frame, (i64, i64)),
    Append(Frame),
    Compose,
    Capture,
    Preview,
    ScrollCapture(bool),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CaptureMode {
    Current,
    ScrollForward,
    ScrollBackward,
}
type Capture = Box<dyn CaptureSource>;
pub struct SessionJob {
    sender: Option<mpsc::SyncSender<Command>>,
    receiver: Option<mpsc::Receiver<Reply>>,
    worker: Option<JoinHandle<()>>,
    cancel: Arc<AtomicBool>,
    pending: bool,
    snapshot: Snapshot,
    has_capture: bool,
    has_scroll: bool,
    has_move: bool,
}
impl SessionJob {
    /// Session 在调用线程已完成参数准入；此处仅移交所有权，不复制输入像素。
    pub fn start(session: Session) -> std::io::Result<Self> {
        Self::start_inner(session, None)
    }
    /// 用户驱动录制从未知轴开始；采集源不获得发送滚轮的能力。
    pub fn start_detecting(
        first: Frame,
        source: impl CaptureSource + 'static,
    ) -> std::io::Result<Self> {
        Self::start_active(
            active::Active::Detecting(detect::Detect::new(first)),
            Some(Box::new(source)),
        )
    }
    /// 采集源只在 worker 收到显式命令时运行；不得在闭包中读取 UI 可变状态。
    pub fn start_with_capture(
        session: Session,
        mut capture: impl FnMut(&AtomicBool) -> Result<Frame, String> + Send + 'static,
    ) -> std::io::Result<Self> {
        Self::start_inner(
            session,
            Some(Box::new(source::CallbackSource {
                callback: Box::new(move |_, cancel| capture(cancel)),
                scroll: false,
            })),
        )
    }
    pub fn start_with_actions(
        session: Session,
        capture: impl FnMut(CaptureMode, &AtomicBool) -> Result<Frame, String> + Send + 'static,
    ) -> std::io::Result<Self> {
        Self::start_inner(
            session,
            Some(Box::new(source::CallbackSource {
                callback: Box::new(capture),
                scroll: true,
            })),
        )
    }
    pub fn start_with_source(
        session: Session,
        source: impl CaptureSource + 'static,
    ) -> std::io::Result<Self> {
        Self::start_inner(session, Some(Box::new(source)))
    }
    fn start_inner(session: Session, capture: Option<Capture>) -> std::io::Result<Self> {
        Self::start_active(active::Active::Linear(session), capture)
    }
    fn start_active(session: active::Active, capture: Option<Capture>) -> std::io::Result<Self> {
        let has_capture = capture.is_some();
        let has_scroll = capture.as_ref().is_some_and(|source| source.can_scroll());
        let has_move = capture.as_ref().is_some_and(|source| source.can_move());
        let snapshot = session.snapshot();
        let (sender, commands) = mpsc::sync_channel(1);
        let (results, receiver) = mpsc::sync_channel(1);
        let cancel = Arc::new(AtomicBool::new(false));
        let signal = cancel.clone();
        let worker = std::thread::Builder::new()
            .name("otter-long-capture".into())
            .spawn(move || run(session, commands, results, signal, capture))?;
        Ok(Self {
            sender: Some(sender),
            receiver: Some(receiver),
            worker: Some(worker),
            cancel,
            pending: false,
            snapshot,
            has_capture,
            has_scroll,
            has_move,
        })
    }
    pub fn snapshot(&self) -> Snapshot {
        self.snapshot
    }
    pub fn can_scroll(&self) -> bool {
        self.has_scroll
    }
    pub fn can_move(&self) -> bool {
        self.has_move
    }
    pub fn move_capture(&mut self, dx: i32, dy: i32) -> Result<(), DispatchError> {
        if !self.has_move {
            return Err(DispatchError::Unavailable);
        }
        self.dispatch(Command::MoveCapture((dx, dy)))
    }
    pub fn scroll_capture(&mut self, forward: bool) -> Result<(), DispatchError> {
        if !self.has_scroll {
            return Err(DispatchError::Unavailable);
        }
        self.dispatch(Command::ScrollCapture(forward))
    }
    pub fn is_finished(&self) -> bool {
        self.worker
            .as_ref()
            .is_none_or(|worker| worker.is_finished())
    }
    pub fn is_pending(&self) -> bool {
        self.pending
    }
    pub fn append(&mut self, frame: Frame) -> Result<(), DispatchError> {
        self.dispatch(Command::Append(frame))
    }
    /// frame 已由采集源获得；movement 始终相对最后接受帧，失败不能推进调用方基准。
    pub fn append_at(&mut self, frame: Frame, movement: (i64, i64)) -> Result<(), DispatchError> {
        self.dispatch(Command::AppendAt(frame, movement))
    }
    pub fn compose(&mut self) -> Result<(), DispatchError> {
        self.dispatch(Command::Compose)
    }
    pub fn preview(&mut self) -> Result<(), DispatchError> {
        self.dispatch(Command::Preview)
    }
    pub fn capture(&mut self) -> Result<(), DispatchError> {
        if !self.has_capture {
            return Err(DispatchError::Unavailable);
        }
        self.dispatch(Command::Capture)
    }
    fn dispatch(&mut self, command: Command) -> Result<(), DispatchError> {
        if self.cancel.load(Ordering::Acquire) || self.sender.is_none() {
            return Err(DispatchError::Stopped);
        }
        if self.pending {
            return Err(DispatchError::Busy);
        }
        match self.sender.as_ref().unwrap().try_send(command) {
            Ok(()) => {
                self.pending = true;
                Ok(())
            }
            Err(mpsc::TrySendError::Full(_)) => Err(DispatchError::Busy),
            Err(mpsc::TrySendError::Disconnected(_)) => Err(DispatchError::Stopped),
        }
    }
    /// 非阻塞消费唯一回复。取消后关闭结果通道，任何迟到图像都不能重新发布。
    pub fn take_ready(&mut self) -> Option<Reply> {
        if !self.pending || self.cancel.load(Ordering::Acquire) {
            return None;
        }
        let result = self.receiver.as_ref()?.try_recv();
        let reply = match result {
            Ok(reply) => reply,
            Err(mpsc::TryRecvError::Empty) => return None,
            Err(mpsc::TryRecvError::Disconnected) => Reply {
                can_scroll: false,
                can_move: false,
                snapshot: self.snapshot,
                completion: Completion::WorkerStopped,
            },
        };
        self.pending = false;
        self.snapshot = reply.snapshot;
        self.has_scroll = reply.can_scroll;
        self.has_move = reply.can_move;
        Some(reply)
    }
    /// 不等待 worker；等待由最终 Drop 负责。所有在途/已排队结果即刻失去发布通道。
    pub fn cancel(&mut self) {
        self.cancel.store(true, Ordering::Release);
        self.sender.take();
        self.receiver.take();
        self.pending = false;
    }
}
impl Drop for SessionJob {
    fn drop(&mut self) {
        self.cancel();
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}
fn snapshot(session: &Session) -> Snapshot {
    Snapshot {
        axis: Some(session.axis()),
        direction: session
            .motion()
            .and_then(|delta| Direction::from_offset(session.axis(), delta)),
        spatial_position: None,
        frame_count: session.frame_count(),
        confirmed_edges: session.confirmed_edges(),
    }
}
fn run(
    mut session: active::Active,
    commands: mpsc::Receiver<Command>,
    results: mpsc::SyncSender<Reply>,
    cancel: Arc<AtomicBool>,
    mut capture: Option<Capture>,
) {
    while let Ok(command) = commands.recv() {
        if cancel.load(Ordering::Acquire) {
            break;
        }
        let completion = match command {
            Command::MoveCapture(movement) => match capture.as_mut() {
                Some(source) if source.can_move() => {
                    let result = match source.prepare_move(movement, &cancel) {
                        Ok(frame) => session.append_at(
                            frame,
                            (i64::from(movement.0), i64::from(movement.1)),
                            &cancel,
                        ),
                        Err(error) => Completion::CaptureFailed(error),
                    };
                    let accepted = !cancel.load(Ordering::Acquire)
                        && matches!(
                            result,
                            Completion::Append(Ok(Append::Added { .. } | Append::Duplicate))
                                | Completion::SpatialAppend(Ok(
                                    SpatialAppend::Added { .. } | SpatialAppend::Duplicate
                                ))
                        );
                    source.finish_move(accepted);
                    result
                }
                _ => Completion::CaptureFailed("Viewfinder source unavailable".into()),
            },
            Command::AppendAt(frame, movement) => session.append_at(frame, movement, &cancel),
            Command::Append(frame) => session.append(frame, &cancel),
            Command::Compose => session.compose(&cancel),
            Command::Preview => session.preview(&cancel),
            Command::ScrollCapture(forward) => {
                if let Err(error) = session.admit_next() {
                    Completion::Append(Err(error))
                } else {
                    match capture.as_mut() {
                        Some(source) if source.can_scroll() => match source.capture(
                            if forward {
                                CaptureMode::ScrollForward
                            } else {
                                CaptureMode::ScrollBackward
                            },
                            &cancel,
                        ) {
                            Ok(frame) => session.append(frame, &cancel),
                            Err(error) => Completion::CaptureFailed(error),
                        },
                        _ => Completion::CaptureFailed("Scroll source unavailable".into()),
                    }
                }
            }
            Command::Capture => match capture.as_mut() {
                Some(source) => match source.capture(CaptureMode::Current, &cancel) {
                    Ok(frame) => session.append(frame, &cancel),
                    Err(error) => Completion::CaptureFailed(error),
                },
                None => Completion::CaptureFailed("Capture source unavailable".into()),
            },
        };
        if cancel.load(Ordering::Acquire) {
            break;
        }
        let reply = Reply {
            can_scroll: capture.as_ref().is_some_and(|source| source.can_scroll()),
            can_move: capture.as_ref().is_some_and(|source| source.can_move()),
            snapshot: session.snapshot(),
            completion,
        };
        // 单在途协议使结果槽始终为空；非阻塞发送仍防止异常调用造成退出死锁。
        if results.try_send(reply).is_err() {
            break;
        }
    }
}

#[cfg(test)]
#[path = "long_capture_detect_job_tests.rs"]
mod detecting_tests;
#[cfg(test)]
#[path = "long_capture_job_tests.rs"]
mod tests;

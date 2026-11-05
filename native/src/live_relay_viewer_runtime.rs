//! 单代次后台观看任务：网络线程拥有连接与 codec worker，UI 只停止并等待完成句柄。
use crate::{
    byte_budget::Budget,
    live_relay_media_endpoint::Prepared,
    live_relay_media_socket::Step,
    live_relay_media_transport::{Connection, Error as TransportError},
    live_relay_media_worker::{Exit as WorkerExit, Worker},
    live_relay_viewer_session::{Permit, ViewerSession},
    loom_bridge_stream::Interrupt,
};
use std::{
    io,
    net::SocketAddr,
    os::windows::io::{AsHandle, BorrowedHandle},
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc, Mutex,
    },
    thread::JoinHandle,
};
#[path = "live_relay_viewer_reconnect.rs"]
mod reconnect;
pub(super) use reconnect::Recovery;
#[cfg(test)]
pub(crate) struct Disconnect(std::sync::Weak<Control>);
#[cfg(test)]
impl Disconnect {
    pub(crate) fn run(&self) -> bool {
        self.0.upgrade().is_some_and(|control| {
            control
                .interrupt
                .lock()
                .unwrap()
                .as_ref()
                .is_some_and(|interrupt| interrupt.cancel().is_ok())
        })
    }
}

static ACTIVE: AtomicUsize = AtomicUsize::new(0);
struct Slot;
impl Slot {
    fn acquire() -> io::Result<Self> {
        ACTIVE
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |n| {
                (n < 8).then_some(n + 1)
            })
            .map_err(|_| {
                io::Error::new(io::ErrorKind::WouldBlock, "Viewer runtime limit reached")
            })?;
        Ok(Self)
    }
}
impl Drop for Slot {
    fn drop(&mut self) {
        ACTIVE.fetch_sub(1, Ordering::AcqRel);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Exit {
    Stopped,
    Closed,
    DeviceRevoked,
    Transport(TransportError),
    WorkerStart,
    Worker(WorkerExit),
    Panicked,
    AuthorizationEnded,
}

struct Control {
    stop: AtomicBool,
    interrupt: Mutex<Option<Interrupt>>,
    session: Arc<ViewerSession>,
    permit: Permit,
    worker: Mutex<Option<std::thread::Thread>>,
}
impl Control {
    fn install(&self, interrupt: Interrupt) {
        let mut slot = self.interrupt.lock().unwrap_or_else(|e| e.into_inner());
        if self.stop.load(Ordering::Acquire) {
            let _ = interrupt.cancel();
        } else {
            *slot = Some(interrupt);
        }
    }
    fn stop(&self) {
        self.stop.store(true, Ordering::Release);
        // 先撤销像素发布，再中断 socket；只影响原许可，旧任务不能暂停新代次。
        let _ = self.session.suspend_if_current(&self.permit);
        let handle = self
            .interrupt
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .take();
        if let Some(handle) = handle {
            let _ = handle.cancel();
        }
        if let Some(worker) = self
            .worker
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .as_ref()
        {
            worker.unpark();
        }
    }
}
struct Retire(Arc<Control>);
impl Drop for Retire {
    fn drop(&mut self) {
        self.0.stop();
    }
}

pub struct Runtime {
    control: Arc<Control>,
    thread: Option<JoinHandle<Exit>>,
}
impl Runtime {
    #[cfg(test)]
    pub(crate) fn test_disconnect(&self) -> Disconnect {
        Disconnect(Arc::downgrade(&self.control))
    }
    /// session/permit 必须已通过控制流授权与 bootstrap；本方法不自动激活会话。
    pub fn start(
        prepared: Prepared,
        addresses: Vec<SocketAddr>,
        session: Arc<ViewerSession>,
        permit: Permit,
        budget: Arc<Budget>,
    ) -> io::Result<Self> {
        Self::start_observed(prepared, addresses, session, permit, budget, |_| {})
    }
    /// 完成回调在网络线程运行，必须有界且不等待 UI；先保存终态再唤醒其他 owner。
    pub(super) fn start_observed(
        prepared: Prepared,
        addresses: Vec<SocketAddr>,
        session: Arc<ViewerSession>,
        permit: Permit,
        budget: Arc<Budget>,
        completed: impl FnOnce(Exit) + Send + 'static,
    ) -> io::Result<Self> {
        Self::start_recovering(
            prepared, addresses, session, permit, budget, None, completed,
        )
    }
    pub(super) fn start_recovering(
        prepared: Prepared,
        addresses: Vec<SocketAddr>,
        session: Arc<ViewerSession>,
        permit: Permit,
        budget: Arc<Budget>,
        recovery: Option<Recovery>,
        completed: impl FnOnce(Exit) + Send + 'static,
    ) -> io::Result<Self> {
        session.validate(&permit).map_err(|_| {
            io::Error::new(io::ErrorKind::PermissionDenied, "Viewer permit invalid")
        })?;
        if addresses.is_empty() || addresses.len() > 4 {
            return Err(io::ErrorKind::InvalidInput.into());
        }
        if !session.matches_media_target(&prepared.session_id, &prepared.device_id) {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "Viewer media target mismatch",
            ));
        }
        let slot = Slot::acquire()?;
        let control = Arc::new(Control {
            stop: AtomicBool::new(false),
            interrupt: Mutex::new(None),
            session,
            permit,
            worker: Mutex::new(None),
        });
        let retired = Retire(control.clone());
        let thread = std::thread::Builder::new()
            .name("otter-live-network".into())
            .spawn(move || {
                let _slot = slot;
                let _retired = retired;
                *_retired.0.worker.lock().unwrap_or_else(|e| e.into_inner()) =
                    Some(std::thread::current());
                let exit = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    run(prepared, &addresses, &_retired.0, &budget, recovery)
                }))
                .unwrap_or(Exit::Panicked);
                completed(exit);
                exit
            })?;
        Ok(Self {
            control,
            thread: Some(thread),
        })
    }
    pub fn request_stop(&self) {
        self.control.stop();
    }
    pub fn wait_handle(&self) -> Option<BorrowedHandle<'_>> {
        self.thread.as_ref().map(|t| t.as_handle())
    }
    pub fn try_join(&mut self) -> Option<Exit> {
        if !self.thread.as_ref()?.is_finished() {
            return None;
        }
        Some(self.thread.take()?.join().unwrap_or(Exit::Panicked))
    }
}
impl Drop for Runtime {
    fn drop(&mut self) {
        self.request_stop();
        // UI 正常路径必须先等待完成并 try_join；这里只是防遗留线程的阻塞兜底。
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn run(
    mut prepared: Prepared,
    addresses: &[SocketAddr],
    control: &Control,
    budget: &Arc<Budget>,
    mut recovery: Option<Recovery>,
) -> Exit {
    let mut worker = None;
    let mut cursor = crate::live_relay_viewer_frames::Cursor {
        epoch: control.permit.epoch(),
        frame_id: 0,
    };
    loop {
        if control.stop.load(Ordering::Acquire)
            || control.session.validate(&control.permit).is_err()
        {
            return Exit::Stopped;
        }
        if !control
            .session
            .matches_media_target(&prepared.session_id, &prepared.device_id)
        {
            return Exit::AuthorizationEnded;
        }
        let result =
            match Connection::connect_cancellable(prepared, addresses, &control.stop, |handle| {
                control.install(handle)
            }) {
                Ok(mut connection) => {
                    if worker.is_none() {
                        worker =
                            match Worker::start(control.session.clone(), control.permit.clone()) {
                                Ok(worker) => Some(worker),
                                Err(_) => return Exit::WorkerStart,
                            };
                    }
                    service(
                        &mut connection,
                        worker.as_mut().unwrap(),
                        control,
                        budget,
                        &mut recovery,
                        &mut cursor,
                    )
                }
                Err(error) => Exit::Transport(error),
            };
        // 关闭原socket及其interrupt后才重建；一个Runtime始终只有一个network/codec线程。
        control
            .interrupt
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .take();
        if control.stop.load(Ordering::Acquire) {
            return Exit::Stopped;
        }
        let Some(recovery) = recovery.as_mut() else {
            return result;
        };
        prepared = match recovery.retry(result, control, cursor) {
            Ok(prepared) => prepared,
            Err(exit) => return exit,
        };
        if worker.as_ref().is_some_and(Worker::prefers_legacy) {
            prepared.viewer_legacy();
        }
    }
    // worker 的 Drop 在此网络线程停止/join codec 线程；UI 不参与阻塞回收。
}

fn service(
    connection: &mut Connection,
    worker: &mut Worker,
    control: &Control,
    budget: &Arc<Budget>,
    recovery: &mut Option<Recovery>,
    cursor: &mut crate::live_relay_viewer_frames::Cursor,
) -> Exit {
    // 只忽略本次连接最初的旧图像预热；新帧出现后重复/回退仍由原pipeline失败关闭。
    let mut warmup = Some(*cursor);
    loop {
        if control.stop.load(Ordering::Acquire) {
            return Exit::Stopped;
        }
        if let Some(exit) = worker.try_join() {
            return Exit::Worker(exit);
        }
        let result = if recovery.is_some() {
            connection.service_resumable(budget, worker, warmup)
        } else {
            connection.service(budget, worker).map(|step| (step, None))
        };
        match result {
            Ok((Step::Idle | Step::Queued, observed)) => {
                if let Some(metadata) = observed.filter(|m| m.epoch == cursor.epoch) {
                    cursor.frame_id = cursor.frame_id.max(metadata.frame_id);
                    if warmup.is_some_and(|floor| metadata.frame_id > floor.frame_id) {
                        warmup = None;
                    }
                }
                if let Some(recovery) = recovery.as_mut() {
                    let Ok(committed) = control.session.committed(&control.permit) else {
                        return Exit::Stopped;
                    };
                    recovery.progress(committed);
                }
            }
            Ok((Step::Closed, _)) => return Exit::Closed,
            Ok((Step::DeviceRevoked, _)) => return Exit::DeviceRevoked,
            Err(error) => return Exit::Transport(error),
        }
    }
}

#[cfg(test)]
#[path = "live_relay_viewer_runtime_tests.rs"]
mod tests;

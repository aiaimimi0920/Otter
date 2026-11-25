//! 一条串行Source后台链；UI不做DNS/HTTP/GPU读回，也不因停止而绕过原授权关闭会话。
use crate::{
    byte_budget::Budget,
    device_authorization_config::Config,
    device_authorization_lease::{ExecutionStop, Lease},
    live_frames::Consumer,
    live_relay_http::{self, source::Publication},
    live_texture::Texture,
    loom_bridge_stream::Interrupt,
};
use std::{
    io,
    os::windows::io::{AsHandle, AsRawHandle, BorrowedHandle, FromRawHandle, OwnedHandle},
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc, Mutex,
    },
    thread::JoinHandle,
};
use windows::Win32::{
    Foundation::HANDLE,
    System::Threading::{CreateEventW, ResetEvent, SetEvent},
};
#[path = "live_relay_source_control.rs"]
mod control;
#[path = "live_relay_source_input.rs"]
pub(crate) mod input;
#[path = "live_relay_source_reconnect.rs"]
mod recovery;
#[path = "live_relay_source_stream.rs"]
mod stream;
#[path = "live_relay_source_video.rs"]
mod video;
#[path = "live_relay_source_input_wire.rs"]
mod wire;
pub(crate) use control::Interaction;
#[path = "live_source_observation_runtime.rs"]
mod observation;
pub(crate) use observation::Observation;
#[cfg(test)]
#[path = "live_relay_source_test_texture.rs"]
pub(crate) mod test_texture;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Exit {
    Stopped,
    SourceClosed,
    Lease,
    Geometry,
    Budget,
    Readback,
    Preparation(crate::device_authorization_config::Error),
    Http(live_relay_http::Error),
    MediaStart(crate::device_authorization_lease::MediaError),
    Media(crate::live_relay_media_transport::Error),
    RemoteClosed,
    Revoked,
    Panicked,
    Control(control::Error),
    Observation(observation::Error),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cleanup {
    NotCreated,
    Confirmed,
    Unconfirmed,
    AuthorizationEnded,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Outcome {
    pub exit: Exit,
    pub cleanup: Cleanup,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Status {
    Starting,
    Publishing,
    Recovering,
    Stopping,
}
struct Signal {
    stop: AtomicBool,
    progress_stop: ExecutionStop,
    socket: Mutex<Option<Interrupt>>,
    worker: Mutex<Option<std::thread::Thread>>,
    ready: AtomicBool,
    recovering: AtomicBool,
    control_recovering: AtomicBool,
    observation_recovering: AtomicBool,
    restart_media: AtomicBool,
    generation: AtomicU64,
    event: OwnedHandle,
    interaction: Option<Arc<Interaction>>,
    observation: Option<Arc<Observation>>,
}
impl Signal {
    fn service_recovery(&self, state: &AtomicBool, recovering: bool) {
        if state.swap(recovering, Ordering::AcqRel) != recovering {
            let _ = unsafe { SetEvent(HANDLE(self.event.as_raw_handle())) };
        }
    }
    fn restart_media(&self) {
        // 控制失联必须在服务产生可靠controller=None屏障；仅由原媒体owner重连，不新增写请求。
        // socket尚未安装时仍保留请求，避免首次控制GET先于WSS建立的竞争。
        {
            // 发布请求与选择取消对象共用连接槽；不能中断随后才安装的替代连接。
            let owner = self.socket.lock().unwrap_or_else(|e| e.into_inner());
            self.recovery(true);
            self.restart_media.store(true, Ordering::Release);
            if let Some(socket) = owner.as_ref() {
                let _ = socket.cancel();
            }
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
    fn recovery(&self, recovering: bool) {
        if self.recovering.swap(recovering, Ordering::AcqRel) != recovering {
            if recovering {
                self.generation.fetch_add(1, Ordering::AcqRel);
            }
            let _ = unsafe { SetEvent(HANDLE(self.event.as_raw_handle())) };
        }
    }
    fn stop(&self) {
        self.stop.store(true, Ordering::Release);
        // 只中断水位 GET；显式停止后仍须用本租约关闭原会话。
        self.progress_stop.stop();
        if let Some(interaction) = &self.interaction {
            interaction.cancel();
        }
        if let Some(observation) = &self.observation {
            observation.cancel();
        }
        if let Some(socket) = self
            .socket
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .as_ref()
        {
            let _ = socket.cancel();
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
    fn finish_media_recovery(&self) -> bool {
        // 与轮换请求串行：旧连接返回的水位不能解除本次输入屏障。
        let _owner = self.socket.lock().unwrap_or_else(|e| e.into_inner());
        if self.restart_media.load(Ordering::Acquire) {
            return false;
        }
        self.recovery(false);
        true
    }
    fn retire_media(&self) {
        self.socket.lock().unwrap_or_else(|e| e.into_inner()).take();
    }
    fn install(&self, socket: Interrupt) {
        let mut owner = self.socket.lock().unwrap_or_else(|e| e.into_inner());
        if self.stop.load(Ordering::Acquire) {
            let _ = socket.cancel();
        } else {
            *owner = Some(socket);
        }
    }
}
pub struct Runtime {
    signal: Arc<Signal>,
    thread: Option<JoinHandle<Outcome>>,
    slot: Option<crate::live_relay_control_runtime::Slot>,
}
impl Runtime {
    #[cfg(test)]
    pub(crate) fn test_finished(&self) -> bool {
        self.thread
            .as_ref()
            .is_none_or(|thread| thread.is_finished())
    }
    pub fn start(
        config: Config,
        lease: Lease,
        publication: Publication,
        consumer: Consumer<Texture>,
        budget: Arc<Budget>,
    ) -> io::Result<Self> {
        Self::start_with_interaction(config, lease, publication, consumer, budget, None)
    }
    pub(crate) fn start_with_interaction(
        config: Config,
        lease: Lease,
        publication: Publication,
        consumer: Consumer<Texture>,
        budget: Arc<Budget>,
        interaction: Option<Arc<Interaction>>,
    ) -> io::Result<Self> {
        Self::start_with_observation(
            config,
            lease,
            publication,
            consumer,
            budget,
            interaction,
            None,
        )
    }
    pub(crate) fn start_with_observation(
        config: Config,
        lease: Lease,
        publication: Publication,
        consumer: Consumer<Texture>,
        budget: Arc<Budget>,
        interaction: Option<Arc<Interaction>>,
        observation: Option<Arc<Observation>>,
    ) -> io::Result<Self> {
        if publication.interactive() != interaction.is_some()
            || interaction.as_ref().is_some_and(|i| !i.matches(&lease))
            || publication.observing() != observation.is_some()
            || observation
                .as_ref()
                .is_some_and(|o| !o.matches(&lease) || !o.target_matches(&publication))
        {
            return Err(io::ErrorKind::PermissionDenied.into());
        }
        let slot = crate::live_relay_control_runtime::Slot::acquire()?;
        let event = unsafe { CreateEventW(None, true, false, None) }.map_err(io::Error::other)?;
        let signal = Arc::new(Signal {
            stop: AtomicBool::new(false),
            progress_stop: lease.execution_stop_handle(),
            socket: Mutex::new(None),
            worker: Mutex::new(None),
            ready: AtomicBool::new(false),
            recovering: AtomicBool::new(false),
            control_recovering: AtomicBool::new(false),
            observation_recovering: AtomicBool::new(false),
            restart_media: AtomicBool::new(false),
            generation: AtomicU64::new(0),
            event: unsafe { OwnedHandle::from_raw_handle(event.0) },
            interaction,
            observation,
        });
        let weak = Arc::downgrade(&signal);
        lease.notify_revocation(Arc::new(move || {
            if let Some(signal) = weak.upgrade() {
                signal.stop();
            }
        }));
        let worker = signal.clone();
        let thread = std::thread::Builder::new()
            .name("otter-live-source".into())
            .spawn(move || {
                *worker.worker.lock().unwrap_or_else(|e| e.into_inner()) =
                    Some(std::thread::current());
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    run(config, &lease, publication, consumer, &budget, &worker)
                }))
                .unwrap_or(Outcome {
                    exit: Exit::Panicked,
                    cleanup: Cleanup::Unconfirmed,
                });
                // 配置/预授权早退或panic同样撤销输入桥，不能遗留独占的Session输入资格。
                worker.stop();
                worker
                    .socket
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .take();
                worker
                    .worker
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .take();
                result
            })?;
        Ok(Self {
            signal,
            thread: Some(thread),
            slot: Some(slot),
        })
    }
    pub fn request_stop(&self) {
        self.signal.stop();
    }
    pub fn stopping(&self) -> bool {
        self.signal.stop.load(Ordering::Acquire)
    }
    pub fn ready(&self) -> bool {
        self.status() == Status::Publishing
    }
    pub(crate) fn status(&self) -> Status {
        // 先消费事件再观察状态；期间到达的新变化仍可唤醒消息泵。
        let _ = unsafe { ResetEvent(HANDLE(self.signal.event.as_raw_handle())) };
        let ready = self.signal.ready.load(Ordering::Acquire);
        let recovering = self.recovering();
        if self.stopping() {
            Status::Stopping
        } else if recovering {
            Status::Recovering
        } else if ready {
            Status::Publishing
        } else {
            Status::Starting
        }
    }
    pub(crate) fn recovering(&self) -> bool {
        self.signal.recovering.load(Ordering::Acquire)
            || self.signal.control_recovering.load(Ordering::Acquire)
            || self.signal.observation_recovering.load(Ordering::Acquire)
            || self.signal.restart_media.load(Ordering::Acquire)
    }
    #[cfg(test)]
    pub(crate) fn test_disconnect_control(&self) -> bool {
        self.signal
            .interaction
            .as_ref()
            .is_some_and(|i| i.test_disconnect())
    }
    #[cfg(test)]
    pub(crate) fn test_request_media_restart(&self) {
        self.signal.restart_media();
    }
    #[cfg(test)]
    pub(crate) fn test_disconnect_media(&self) -> bool {
        self.signal
            .socket
            .lock()
            .unwrap()
            .as_ref()
            .is_some_and(|s| s.cancel().is_ok())
    }
    pub fn wait_handles(&self) -> impl Iterator<Item = BorrowedHandle<'_>> {
        self.thread
            .iter()
            .map(AsHandle::as_handle)
            // 停止后只等真实线程；Ready不再消费，不能常亮信号忙转或扩张退出等待预算。
            .chain((!self.stopping()).then(|| self.signal.event.as_handle()))
    }
    pub fn try_join(&mut self) -> Option<Outcome> {
        if !self.thread.as_ref()?.is_finished() {
            return None;
        }
        let result = self.thread.take()?.join().unwrap_or(Outcome {
            exit: Exit::Panicked,
            cleanup: Cleanup::Unconfirmed,
        });
        self.slot.take();
        Some(result)
    }
}
impl Drop for Runtime {
    fn drop(&mut self) {
        self.request_stop();
        // 正常退出由Application泵消息回收；这里只是防孤儿线程兜底。
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
pub(crate) fn now() -> Result<u64, Exit> {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .and_then(|v| u64::try_from(v.as_millis()).ok())
        .ok_or(Exit::Lease)
}
fn run(
    config: Config,
    lease: &Lease,
    mut publication: Publication,
    consumer: Consumer<Texture>,
    budget: &Arc<Budget>,
    signal: &Arc<Signal>,
) -> Outcome {
    let client = match lease.prepare_client(&config) {
        Ok(client) => client,
        Err(error) => {
            return Outcome {
                exit: if signal.stop.load(Ordering::Acquire) {
                    Exit::Stopped
                } else {
                    Exit::Preparation(error)
                },
                cleanup: Cleanup::NotCreated,
            }
        }
    };
    if signal.stop.load(Ordering::Acquire) {
        return Outcome {
            exit: Exit::Stopped,
            cleanup: Cleanup::NotCreated,
        };
    }
    // encoder在原媒体线程建立/销毁，成功才广告能力；不支持/预算不足沿原raw创建/关闭。
    let video = video::Video::new(publication.size(), budget);
    publication.set_h264(video.available());
    let publication = Arc::new(publication);
    let mut controller = None;
    let mut observer = None;
    let result = (|| {
        let control = lease.control(&client).map_err(|_| Exit::Lease)?;
        let snapshot = control.publish_source(&publication).map_err(Exit::Http)?;
        if signal.stop.load(Ordering::Acquire) {
            return Err(Exit::Stopped);
        }
        if let Some(interaction) = &signal.interaction {
            controller = Some(
                control::start(
                    client.clone(),
                    snapshot,
                    interaction.clone(),
                    signal.clone(),
                )
                .map_err(|_| Exit::Control(control::Error::Panicked))?,
            );
        }
        if let Some(observation) = &signal.observation {
            observer = Some(
                observation::start(
                    client.clone(),
                    publication.clone(),
                    observation.clone(),
                    signal.clone(),
                )
                .map_err(|_| Exit::Observation(observation::Error::Spawn))?,
            );
        }
        stream::run(
            &client,
            lease,
            &publication,
            consumer,
            budget,
            signal,
            video,
        )
    })();
    let requested = signal.stop.load(Ordering::Acquire);
    signal.stop();
    let control_result =
        controller.map(|thread| thread.join().unwrap_or(Err(control::Error::Panicked)));
    let observation_result =
        observer.map(|thread| thread.join().unwrap_or(Err(observation::Error::Panicked)));
    // 先断媒体，再尝试原身份的有界HTTP关闭。停止不换租约；丢响应不能声称已关闭。
    signal
        .socket
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .take();
    let exit = source_exit(requested, result, control_result, observation_result);
    if exit.authorization_ended() {
        lease.revoke();
    }
    let cleanup = if now()
        .ok()
        .is_none_or(|now| lease.validate(client.origin(), now).is_err())
    {
        Cleanup::AuthorizationEnded
    } else {
        match lease
            .control(&client)
            .map(|control| control.close_source(&publication))
        {
            Ok(Ok(())) => Cleanup::Confirmed,
            Ok(Err(live_relay_http::Error::AuthorizationRequired)) => {
                lease.revoke();
                Cleanup::AuthorizationEnded
            }
            _ if lease.is_revoked() => Cleanup::AuthorizationEnded,
            _ => Cleanup::Unconfirmed,
        }
    };
    Outcome { exit, cleanup }
}

#[path = "live_relay_source_outcome.rs"]
mod outcome;
use outcome::source_exit;

#[cfg(test)]
#[path = "live_relay_source_exit_tests.rs"]
mod exit_tests;

//! 加入与持续控制轮询的后台 owner；UI 只接收一次就绪许可并等待线程完成句柄。
use crate::{
    device_authorization_lease::Lease,
    device_session_client::Client,
    live_relay_control_owner::{Error as OwnerError, Owner},
    live_relay_http::{Error as HttpError, Join, RequesterCursor},
    live_relay_viewer_identity::{ControlEvent, Identity},
    live_relay_viewer_session::{Permit, ViewerSession},
};
use std::{
    io,
    os::windows::io::{AsHandle, AsRawHandle, BorrowedHandle, FromRawHandle, OwnedHandle},
    sync::{
        atomic::{AtomicUsize, Ordering},
        mpsc, Arc,
    },
    thread::JoinHandle,
    time::Duration,
};
use windows::Win32::{
    Foundation::HANDLE,
    System::Threading::{CreateEventW, ResetEvent, SetEvent},
};
#[path = "live_relay_media_completion.rs"]
mod completion;
#[path = "live_relay_viewer_input_runtime.rs"]
pub(crate) mod interaction;
#[path = "live_relay_control_reconnect.rs"]
mod recovery;

static ACTIVE: AtomicUsize = AtomicUsize::new(0);
pub(super) struct Slot;
impl Slot {
    pub(super) fn acquire() -> io::Result<Self> {
        ACTIVE
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |n| {
                (n < 4).then_some(n + 1)
            })
            .map_err(|_| {
                io::Error::new(io::ErrorKind::WouldBlock, "Live control task limit reached")
            })?;
        Ok(Self)
    }
}
impl Drop for Slot {
    fn drop(&mut self) {
        ACTIVE.fetch_sub(1, Ordering::AcqRel);
    }
}

pub struct Target {
    pub session: String,
    pub surface: String,
    pub attachment: String,
    pub expected: Option<Identity>,
    /// 目录选择的身份约束；首次加入允许非成员，续签成员要求仍由 expected 决定。
    pub selected: Option<Identity>,
}
pub(crate) struct LocalArtTarget {
    pub session: String,
    pub selected: Identity,
    pub proof: Arc<crate::live_relay_http::art_prepare::viewer::Proof>,
}
enum RequestTarget {
    Surface(Target),
    LocalArt(LocalArtTarget),
}
impl RequestTarget {
    fn prepare(
        &self,
        control: &crate::live_relay_http::Control<'_>,
    ) -> Result<crate::live_relay_http::Prepared, HttpError> {
        match self {
            Self::Surface(target) => {
                let request = Join {
                    session: &target.session,
                    surface: &target.surface,
                    attachment: &target.attachment,
                    expected: target.expected.as_ref(),
                };
                match target.selected.as_ref() {
                    Some(selected) => control.prepare_selected_viewer(request, selected),
                    None => control.prepare_viewer(request),
                }
            }
            Self::LocalArt(target) => {
                control.prepare_local_art_viewer(&target.session, &target.selected, &target.proof)
            }
        }
    }
}
pub struct Ready {
    pub(crate) input: Arc<interaction::Queue>,
    pub media: Arc<ViewerSession>,
    pub permit: Permit,
    pub baseline: RequesterCursor,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Exit {
    Stopped,
    Closed,
    Http(HttpError),
    Control(OwnerError),
    Lease,
    Receiver,
    Panicked,
    Preparation(crate::device_authorization_config::Error),
    MediaStart(crate::device_authorization_lease::MediaError),
    Media(crate::live_relay_viewer_runtime::Exit),
}
enum Connection {
    Prepared(Client),
    Configured(crate::device_authorization_config::Config),
}
struct Retire(Arc<Lease>);
impl Drop for Retire {
    fn drop(&mut self) {
        self.0.revoke();
    }
}
pub struct Runtime {
    input: Arc<interaction::Queue>,
    lease: Arc<Lease>,
    ready: mpsc::Receiver<Ready>,
    ready_event: Arc<OwnedHandle>,
    thread: Option<JoinHandle<Exit>>,
    // 已完成但未回收的句柄/就绪对象仍占用容量。
    slot: Option<Slot>,
}
impl Runtime {
    pub(crate) fn take_local_trigger(&self) -> Option<crate::art_live_trigger::ticket::Ticket> {
        self.input.local_triggers.take_delivery()
    }
    #[cfg(test)]
    pub(crate) fn thread_finished_for_test(&self) -> bool {
        self.thread.as_ref().is_none_or(|t| t.is_finished())
    }
    #[cfg(test)]
    pub(crate) fn test_disconnect_control(&self) -> bool {
        self.input.test_disconnect_control()
    }
    #[cfg(test)]
    pub(crate) fn test_control(&self) -> (bool, u64) {
        (
            self.input.control_unavailable(),
            self.input.test_cursor.load(Ordering::Acquire),
        )
    }
    #[cfg(test)]
    pub(crate) fn test_disconnect_media(&self) -> bool {
        self.input.test_disconnect_media()
    }
    /// consumer 在后台串行调用；必须处理全部业务事件，不能访问 HWND/UI 线程状态。
    /// consumer 必须有界且不阻塞等待 UI/网络；取消无法强行抢占任意业务回调。
    pub fn start(
        client: Client,
        lease: Lease,
        target: Target,
        consume: impl FnMut(&[ControlEvent]) -> Result<(), ()> + Send + 'static,
    ) -> io::Result<Self> {
        Self::spawn(
            Connection::Prepared(client),
            lease,
            RequestTarget::Surface(target),
            None,
            consume,
        )
    }
    /// 前台只捕获 Config；CA 文件、DNS、HTTPS 均在同一受租约取消的后台线程执行。
    pub fn start_configured(
        config: crate::device_authorization_config::Config,
        lease: Lease,
        target: Target,
        consume: impl FnMut(&[ControlEvent]) -> Result<(), ()> + Send + 'static,
    ) -> io::Result<Self> {
        Self::spawn(
            Connection::Configured(config),
            lease,
            RequestTarget::Surface(target),
            None,
            consume,
        )
    }
    /// 控制重放成功后在后台启动媒体；UI 不持有认证请求或网络线程。
    pub fn start_viewer(
        client: Client,
        lease: Lease,
        target: Target,
        budget: Arc<crate::byte_budget::Budget>,
        consume: impl FnMut(&[ControlEvent]) -> Result<(), ()> + Send + 'static,
    ) -> io::Result<Self> {
        Self::spawn(
            Connection::Prepared(client),
            lease,
            RequestTarget::Surface(target),
            Some(budget),
            consume,
        )
    }
    pub fn start_viewer_configured(
        config: crate::device_authorization_config::Config,
        lease: Lease,
        target: Target,
        budget: Arc<crate::byte_budget::Budget>,
        consume: impl FnMut(&[ControlEvent]) -> Result<(), ()> + Send + 'static,
    ) -> io::Result<Self> {
        Self::spawn(
            Connection::Configured(config),
            lease,
            RequestTarget::Surface(target),
            Some(budget),
            consume,
        )
    }
    pub(crate) fn start_local_art_viewer(
        config: crate::device_authorization_config::Config,
        lease: Lease,
        target: LocalArtTarget,
        budget: Arc<crate::byte_budget::Budget>,
        consume: impl FnMut(&[ControlEvent]) -> Result<(), ()> + Send + 'static,
    ) -> io::Result<Self> {
        Self::spawn(
            Connection::Configured(config),
            lease,
            RequestTarget::LocalArt(target),
            Some(budget),
            consume,
        )
    }
    fn spawn(
        connection: Connection,
        lease: Lease,
        target: RequestTarget,
        budget: Option<Arc<crate::byte_budget::Budget>>,
        mut consume: impl FnMut(&[ControlEvent]) -> Result<(), ()> + Send + 'static,
    ) -> io::Result<Self> {
        let slot = Slot::acquire()?;
        let event = unsafe { CreateEventW(None, true, false, None) }.map_err(io::Error::other)?;
        let ready_event = Arc::new(unsafe { OwnedHandle::from_raw_handle(event.0) });
        let notify = ready_event.clone();
        let lease = Arc::new(lease);
        let retired = Retire(lease.clone());
        let (sender, ready) = mpsc::sync_channel(1);
        let input = Arc::new(interaction::Queue::default());
        let commands = input.clone();
        #[cfg(test)]
        input.test_bind_control(Arc::downgrade(&lease));
        let thread = std::thread::Builder::new()
            .name("otter-live-control".into())
            .spawn(move || {
                let retired = retired;
                let _finish = interaction::Finish(commands.clone());
                let client = match connection {
                    Connection::Prepared(client) => client,
                    Connection::Configured(config) => match retired.0.prepare_client(&config) {
                        Ok(client) => client,
                        Err(_) if retired.0.is_revoked() => return Exit::Stopped,
                        Err(error) => return Exit::Preparation(error),
                    },
                };
                run(
                    &client,
                    &retired.0,
                    target,
                    (sender, notify),
                    budget,
                    &mut consume,
                    commands,
                )
            })?;
        Ok(Self {
            input,
            lease,
            ready,
            ready_event,
            thread: Some(thread),
            slot: Some(slot),
        })
    }
    pub fn request_stop(&self) {
        self.input.stop();
        if !self.input.cleanup_needed() || self.input.control_unavailable() {
            self.lease.revoke();
        }
    }
    pub fn try_ready(&self) -> Option<Ready> {
        // 先 reset 再读取单次通知；生产者稍后 set 最多造成一次空唤醒，不丢就绪。
        let _ = unsafe { ResetEvent(HANDLE(self.ready_event.as_raw_handle())) };
        let ready = self.ready.try_recv().ok()?;
        if self.lease.is_revoked() || ready.media.validate(&ready.permit).is_err() {
            return None;
        }
        Some(ready)
    }
    pub fn wait_handle(&self) -> Option<BorrowedHandle<'_>> {
        self.thread.as_ref().map(|thread| thread.as_handle())
    }
    pub fn ready_handle(&self) -> BorrowedHandle<'_> {
        self.ready_event.as_handle()
    }
    pub fn try_join(&mut self) -> Option<Exit> {
        if !self.thread.as_ref()?.is_finished() {
            return None;
        }
        let result = self.thread.take()?.join().unwrap_or(Exit::Panicked);
        self.slot.take();
        Some(result)
    }
}
impl Drop for Runtime {
    fn drop(&mut self) {
        self.request_stop();
        // 正常应用关闭须先等待完成句柄并回收；此处只作防孤儿线程的阻塞兜底。
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
fn run(
    client: &Client,
    lease: &Arc<Lease>,
    target: RequestTarget,
    notification: (mpsc::SyncSender<Ready>, Arc<OwnedHandle>),
    budget: Option<Arc<crate::byte_budget::Budget>>,
    consume: &mut impl FnMut(&[ControlEvent]) -> Result<(), ()>,
    input: Arc<interaction::Queue>,
) -> Exit {
    let control = match lease.control(client) {
        Ok(control) => control,
        Err(_) => return Exit::Stopped,
    };
    let prepared = match target.prepare(&control) {
        Ok(prepared) => prepared,
        Err(_) if lease.is_revoked() => return Exit::Stopped,
        Err(error) => return Exit::Http(error),
    };
    let mut owner = match Owner::new(prepared, lease.device_id().into()) {
        Ok(owner) => owner,
        Err(error) => return Exit::Control(error),
    };
    if lease.bind_media(&owner.media()).is_err() {
        return Exit::Lease;
    }
    input.configure(owner.business().interaction(), &owner.media());
    input.triggers.configure(match &target {
        RequestTarget::Surface(t) => Some((t.surface.clone(), t.attachment.clone())),
        RequestTarget::LocalArt(_) => None,
    });
    if let RequestTarget::LocalArt(target) = &target {
        input
            .local_triggers
            .configure_host(crate::art_live_trigger::queue::Host {
                binding: target.proof.binding().clone(),
                identity: owner.identity().clone(),
            });
    }
    let mut outgoing = interaction::Sender::new(input.clone(), owner.baseline());
    let (sender, notify) = notification;
    let mut sender = Some(sender);
    let completion = completion::Completion::new(lease.clone());
    // 媒体 Runtime 在控制线程析构，负责停止/回收其网络与 codec 线程，不阻塞 UI。
    let mut media_runtime: Option<crate::live_relay_viewer_runtime::Runtime> = None;
    let mut budget = budget;
    loop {
        if let Some(exit) = completion.exit() {
            return exit;
        }
        if lease.is_revoked() {
            return completion.exit().unwrap_or(Exit::Stopped);
        }
        if let Some(exit) = media_runtime
            .as_mut()
            .and_then(|runtime| runtime.try_join())
        {
            return completion.exit().unwrap_or(Exit::Media(exit));
        }
        let before = owner.after();
        match outgoing.drive_commands(&control, &target, &mut owner) {
            Ok(true) => return Exit::Stopped,
            Ok(false) => {}
            Err(error) => {
                input.finish(true);
                return Exit::Http(error);
            }
        }
        let result = recovery::poll(
            &control,
            lease,
            &input,
            &mut owner,
            consume,
            if outgoing.active() { 50 } else { 750 },
        );
        if let Some(exit) = completion.exit() {
            return exit;
        }
        match result {
            Ok(applied) if applied.closed => return Exit::Closed,
            Ok(applied) => outgoing.observe(applied.controller_owned),
            Err(_) if lease.is_revoked() => return completion.exit().unwrap_or(Exit::Stopped),
            Err(error) => return error,
        }
        #[cfg(test)]
        input.test_cursor.store(owner.after(), Ordering::Release);
        if let Some(sender) = sender.take() {
            let permit = match owner.permit() {
                Ok(permit) => permit,
                Err(error) => return Exit::Control(error),
            };
            if let Some(budget) = budget.take() {
                let completed = completion.clone();
                match lease.start_media_recovering(
                    client,
                    owner.media(),
                    permit.clone(),
                    budget,
                    input.clone(),
                    move |exit| completed.finish(exit),
                ) {
                    Ok(runtime) => media_runtime = Some(runtime),
                    Err(_) if lease.is_revoked() => {
                        return completion.exit().unwrap_or(Exit::Stopped)
                    }
                    Err(error) => return Exit::MediaStart(error),
                }
            }
            if sender
                .try_send(Ready {
                    input: input.clone(),
                    media: owner.media(),
                    permit,
                    baseline: owner.baseline(),
                })
                .is_err()
            {
                return Exit::Receiver;
            }
            let _ = unsafe { SetEvent(HANDLE(notify.as_raw_handle())) };
        }
        // 服务端即使立即返回空批次也不能导致客户端忙循环；取消直接唤醒等待。
        if before == owner.after()
            && !lease.wait(Duration::from_millis(if outgoing.active() {
                10
            } else {
                100
            }))
        {
            return completion.exit().unwrap_or(Exit::Stopped);
        }
    }
}

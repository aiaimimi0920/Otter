//! 新许可执行链的单槽邮箱；事件复位与取值共锁，未领取帧不得覆盖，事件句柄有独立 owner。
use super::*;
use std::{
    os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle},
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Mutex,
    },
};
use windows::Win32::{
    Foundation::HANDLE,
    System::Threading::{CreateEventW, ResetEvent, SetEvent},
};
pub(super) struct Reply {
    pub sequence: u64,
    pub frame: Frame,
}
pub(crate) enum Operation {
    #[cfg(test)]
    Click(String),
    Suspend,
    #[cfg(test)]
    Resume,
    /// 视口与操作同时交付，不能从后台读取 HWND。
    Viewport(crate::surface_viewport::Viewport),
    SelectView(
        crate::art_surface_views::Id,
        crate::surface_viewport::Viewport,
    ),
    Animation(crate::surface_viewport::Viewport),
    Timer(crate::surface_viewport::Viewport),
    Input(
        crate::surface_javascript_wire::LocalInput,
        crate::surface_viewport::Viewport,
    ),
    ClickAt(String, crate::surface_viewport::Viewport),
    LiveTrigger(
        crate::art_live_trigger::ticket::Ticket,
        crate::surface_viewport::Viewport,
    ),
}
pub(super) struct Mailbox {
    command: Mutex<Option<(u64, Operation)>>,
    reply: Mutex<Option<Reply>>,
    command_event: OwnedHandle,
    reply_event: OwnedHandle,
    observed_wall: AtomicU64,
    completed: AtomicBool,
    notify: Arc<dyn Fn() + Send + Sync>,
}
fn event() -> io::Result<OwnedHandle> {
    let handle = unsafe { CreateEventW(None, true, false, None) }.map_err(io::Error::other)?;
    Ok(unsafe { OwnedHandle::from_raw_handle(handle.0) })
}
fn deliver<T>(
    slot: &Mutex<Option<T>>,
    value: T,
    wake: impl FnOnce() -> io::Result<()>,
) -> io::Result<()> {
    let mut slot = slot.lock().map_err(|_| io::ErrorKind::Other)?;
    if slot.is_some() {
        return Err(io::ErrorKind::WouldBlock.into());
    }
    // The receiver needs this same lock before resetting the event or taking
    // the value. A successful wake cannot race ahead of the slot assignment.
    wake()?;
    *slot = Some(value);
    Ok(())
}
impl Mailbox {
    pub fn new(notify: Arc<dyn Fn() + Send + Sync>) -> io::Result<Self> {
        Ok(Self {
            command: Mutex::new(None),
            reply: Mutex::new(None),
            command_event: event()?,
            reply_event: event()?,
            observed_wall: AtomicU64::new(0),
            completed: AtomicBool::new(false),
            notify,
        })
    }
    fn notify(&self) {
        // 回调只允许短投递；异常不能让已发布的完成状态丢失或逃逸到 UI。
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| (self.notify)()));
    }
    pub fn finish(&self) {
        self.completed.store(true, Ordering::Release);
        self.notify();
    }
    pub fn completed(&self) -> bool {
        self.completed.load(Ordering::Acquire)
    }
    pub fn observe(&self, time: u64) {
        self.observed_wall.fetch_max(time, Ordering::AcqRel);
    }
    pub fn observed_wall(&self) -> u64 {
        self.observed_wall.load(Ordering::Acquire)
    }
    pub fn command_handle(&self) -> BorrowedHandle<'_> {
        self.command_event.as_handle()
    }
    pub fn reply_handle(&self) -> BorrowedHandle<'_> {
        self.reply_event.as_handle()
    }
    pub fn submit(&self, sequence: u64, operation: Operation) -> io::Result<()> {
        deliver(&self.command, (sequence, operation), || {
            unsafe { SetEvent(HANDLE(self.command_event.as_raw_handle())) }
                .map_err(io::Error::other)
        })
    }
    pub fn take_command(&self) -> io::Result<Option<(u64, Operation)>> {
        let mut slot = self.command.lock().map_err(|_| io::ErrorKind::Other)?;
        unsafe { ResetEvent(HANDLE(self.command_event.as_raw_handle())) }
            .map_err(io::Error::other)?;
        Ok(slot.take())
    }
    pub fn publish(&self, reply: Reply) -> io::Result<()> {
        deliver(&self.reply, reply, || {
            unsafe { SetEvent(HANDLE(self.reply_event.as_raw_handle())) }.map_err(io::Error::other)
        })?;
        self.notify();
        Ok(())
    }
    pub fn take_reply(&self) -> io::Result<Option<Reply>> {
        let mut slot = self.reply.lock().map_err(|_| io::ErrorKind::Other)?;
        unsafe { ResetEvent(HANDLE(self.reply_event.as_raw_handle())) }
            .map_err(io::Error::other)?;
        Ok(slot.take())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use windows::Win32::{Foundation::WAIT_OBJECT_0, System::Threading::WaitForSingleObject};

    #[test]
    fn failed_wake_leaves_nothing_to_take_and_does_not_replace_an_occupied_slot() {
        let mailbox = Mailbox::new(Arc::new(|| {})).unwrap();
        let error = deliver(&mailbox.command, (1, Operation::Suspend), || {
            assert!(matches!(
                mailbox.command.try_lock(),
                Err(std::sync::TryLockError::WouldBlock)
            ));
            Err(io::ErrorKind::BrokenPipe.into())
        })
        .unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::BrokenPipe);
        assert!(mailbox.take_command().unwrap().is_none());

        mailbox.submit(2, Operation::Resume).unwrap();
        let handle = HANDLE(mailbox.command_handle().as_raw_handle());
        assert_eq!(unsafe { WaitForSingleObject(handle, 0) }, WAIT_OBJECT_0);
        let error = deliver(&mailbox.command, (3, Operation::Suspend), || {
            panic!("occupied slot must not signal a second command")
        })
        .unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::WouldBlock);
        assert!(matches!(
            mailbox.take_command().unwrap(),
            Some((2, Operation::Resume))
        ));
        assert!(mailbox.take_command().unwrap().is_none());
    }
}

//! 单槽 command/reply；事件复位与邮箱检查共锁，禁止覆盖未消费帧或丢失并发唤醒。
use super::*;
use crate::surface_live_binding::Binding;
use std::{
    os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle},
    sync::{
        atomic::{AtomicU64, Ordering},
        Mutex,
    },
};
use windows::Win32::{
    Foundation::HANDLE,
    System::Threading::{CreateEventW, ResetEvent, SetEvent},
};
pub(super) struct Reply {
    pub sequence: u64,
    pub binding: Binding,
    pub revision: u64,
    pub frame: Option<Frame>,
    pub snapshot: Box<serde_json::value::RawValue>,
}
pub(super) enum Operation {
    Rebind,
    Click(String, crate::surface_viewport::Viewport),
    LocalInput(
        crate::surface_javascript_wire::LocalInput,
        crate::surface_viewport::Viewport,
    ),
    Update(crate::surface_viewport::Viewport),
    AnimationFrame(crate::surface_viewport::Viewport),
    TimerTick(crate::surface_viewport::Viewport),
}
pub(super) struct Shared {
    command: Mutex<Option<(u64, MountedSurface, Operation)>>,
    reply: Mutex<(Option<Reply>, bool)>,
    command_event: OwnedHandle,
    reply_event: OwnedHandle,
    expires: AtomicU64,
    #[cfg(test)]
    pub probe: Mutex<Option<Arc<crate::bounded_worker_process::Probe>>>,
}
fn event() -> io::Result<OwnedHandle> {
    let handle = unsafe { CreateEventW(None, true, false, None) }.map_err(io::Error::other)?;
    Ok(unsafe { OwnedHandle::from_raw_handle(handle.0) })
}
impl Shared {
    pub fn new(expires: u64) -> io::Result<Self> {
        Ok(Self {
            command: Mutex::new(None),
            reply: Mutex::new((None, false)),
            command_event: event()?,
            reply_event: event()?,
            expires: AtomicU64::new(expires),
            #[cfg(test)]
            probe: Mutex::new(None),
        })
    }
    pub fn expires(&self) -> u64 {
        self.expires.load(Ordering::Acquire)
    }
    pub fn set_expires(&self, expires: u64) {
        self.expires.store(expires, Ordering::Release);
    }
    pub fn command_handle(&self) -> BorrowedHandle<'_> {
        self.command_event.as_handle()
    }
    pub fn reply_handle(&self) -> BorrowedHandle<'_> {
        self.reply_event.as_handle()
    }
    pub fn submit(
        &self,
        sequence: u64,
        current: MountedSurface,
        operation: Operation,
    ) -> io::Result<()> {
        let mut command = self.command.lock().map_err(|_| io::ErrorKind::Other)?;
        if command.is_some() {
            return Err(io::ErrorKind::WouldBlock.into());
        }
        *command = Some((sequence, current, operation));
        unsafe { SetEvent(HANDLE(self.command_event.as_raw_handle())) }.map_err(io::Error::other)
    }
    pub fn take_command(&self) -> io::Result<Option<(u64, MountedSurface, Operation)>> {
        let mut command = self.command.lock().map_err(|_| io::ErrorKind::Other)?;
        unsafe { ResetEvent(HANDLE(self.command_event.as_raw_handle())) }
            .map_err(io::Error::other)?;
        Ok(command.take())
    }
    pub fn publish(&self, reply: Reply) -> io::Result<()> {
        let mut pending = self.reply.lock().map_err(|_| io::ErrorKind::Other)?;
        if pending.0.is_some() {
            return Err(io::ErrorKind::WouldBlock.into());
        }
        pending.0 = Some(reply);
        unsafe { SetEvent(HANDLE(self.reply_event.as_raw_handle())) }.map_err(io::Error::other)
    }
    pub fn take_reply(&self) -> io::Result<Option<Reply>> {
        let mut pending = self.reply.lock().map_err(|_| io::ErrorKind::Other)?;
        if !pending.1 {
            unsafe { ResetEvent(HANDLE(self.reply_event.as_raw_handle())) }
                .map_err(io::Error::other)?;
        }
        Ok(pending.0.take())
    }
    pub fn has_reply(&self) -> bool {
        self.reply
            .lock()
            .map_or(true, |pending| pending.0.is_some())
    }
    /// DOM 交付和收据结算可落在不同 UI 结算点；复用回复句柄保留最后唤醒，不增加等待槽。
    pub fn hold_notification(&self, hold: bool) -> io::Result<()> {
        let mut pending = self.reply.lock().map_err(|_| io::ErrorKind::Other)?;
        pending.1 = hold;
        unsafe {
            if hold || pending.0.is_some() {
                SetEvent(HANDLE(self.reply_event.as_raw_handle()))
            } else {
                ResetEvent(HANDLE(self.reply_event.as_raw_handle()))
            }
        }
        .map_err(io::Error::other)
    }
}

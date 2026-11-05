//! 帧/许可变化的合并通知；调用者必须在同一媒体状态锁内 signal/reset，避免丢唤醒。
#[cfg(windows)]
use std::os::windows::io::{AsHandle, AsRawHandle, BorrowedHandle, FromRawHandle, OwnedHandle};
#[cfg(windows)]
use windows::Win32::{
    Foundation::HANDLE,
    System::Threading::{CreateEventW, ResetEvent, SetEvent},
};

pub(super) struct Notice {
    #[cfg(windows)]
    event: OwnedHandle,
}
impl Notice {
    pub(super) fn new() -> std::io::Result<Self> {
        #[cfg(windows)]
        {
            let event =
                unsafe { CreateEventW(None, true, false, None) }.map_err(std::io::Error::other)?;
            Ok(Self {
                event: unsafe { OwnedHandle::from_raw_handle(event.0) },
            })
        }
        #[cfg(not(windows))]
        {
            Ok(Self {})
        }
    }
    pub(super) fn signal(&self) {
        #[cfg(windows)]
        let _ = unsafe { SetEvent(HANDLE(self.event.as_raw_handle())) };
    }
    pub(super) fn reset(&self) {
        #[cfg(windows)]
        let _ = unsafe { ResetEvent(HANDLE(self.event.as_raw_handle())) };
    }
    #[cfg(windows)]
    pub(super) fn handle(&self) -> BorrowedHandle<'_> {
        self.event.as_handle()
    }
}

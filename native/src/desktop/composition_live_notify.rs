//! Bounded event bridge from Live consumers to HWND; post only connection generations, never raw pointers or image bytes.
use crate::live_frames::{Consumer, Delivery};
use std::{
    marker::PhantomData,
    rc::Rc,
    sync::atomic::{AtomicUsize, Ordering},
};
use windows::{
    core::{Error, Result},
    Win32::{
        Foundation::*,
        System::Threading::{GetCurrentProcessId, GetCurrentThreadId},
        UI::WindowsAndMessaging::*,
    },
};
pub const MESSAGE: u32 = WM_APP + 0x350;
static NEXT: AtomicUsize = AtomicUsize::new(1);
pub struct WindowSignal<T> {
    consumer: Consumer<T>,
    hwnd: HWND,
    token: usize,
    _thread: PhantomData<Rc<()>>,
}
impl<T> WindowSignal<T> {
    /// Cumulative posting failures; no busy retries, retry on the next frame change. Hosts may report this state through existing events.
    pub fn notification_failures(&self) -> u64 {
        self.consumer.notification_failures()
    }
    /// Compensating reads in existing paint/host events, without polling threads or timers.
    pub(super) fn refresh(
        &mut self,
    ) -> std::result::Result<Option<Delivery<T>>, crate::live_frames::Error> {
        self.consumer.try_next()
    }
    /// Bind on the target HWND's owning thread; destroy owner with the window, and reject old queued messages by new connection generation.
    pub fn bind(hwnd: HWND, mut consumer: Consumer<T>) -> Result<Self> {
        unsafe {
            let mut process = 0;
            let thread = GetWindowThreadProcessId(hwnd, Some(&mut process));
            if !IsWindow(Some(hwnd)).as_bool()
                || process != GetCurrentProcessId()
                || thread != GetCurrentThreadId()
            {
                return Err(Error::new(
                    E_INVALIDARG,
                    "Live notification requires an owned HWND on this thread",
                ));
            }
        }
        let token = NEXT
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |value| {
                value.checked_add(1)
            })
            .map_err(|_| Error::new(E_FAIL, "Live notification identity exhausted"))?;
        let address = hwnd.0 as usize;
        consumer
            .set_waker(move || unsafe {
                PostMessageW(
                    Some(HWND(address as *mut _)),
                    MESSAGE,
                    WPARAM(token),
                    LPARAM(0),
                )
                .is_ok()
            })
            .map_err(|error| {
                Error::new(E_FAIL, format!("Live notification unavailable: {error:?}"))
            })?;
        Ok(Self {
            consumer,
            hwnd,
            token,
            _thread: PhantomData,
        })
    }
    /// None means the message is not for this connection; messages only wake reads, whose actual data always comes from the bound consumer.
    pub fn receive(
        &mut self,
        hwnd: HWND,
        message: u32,
        wp: WPARAM,
        lp: LPARAM,
    ) -> Option<std::result::Result<Option<Delivery<T>>, crate::live_frames::Error>> {
        if hwnd != self.hwnd || message != MESSAGE || wp.0 != self.token || lp.0 != 0 {
            return None;
        }
        Some(self.consumer.try_next())
    }
}
#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-live")))]
#[path = "composition_live_notify_tests.rs"]
mod tests;

//! The sole HWND timer while a track is held; no catch-up for accumulated ticks or timer-identity reuse across gestures.
use std::{
    sync::atomic::{AtomicUsize, Ordering},
    time::{Duration, Instant},
};
use windows::{
    core::{Error, Result},
    Win32::{Foundation::*, UI::WindowsAndMessaging::*},
};
const DELAY_MS: u32 = 400;
const INTERVAL_MS: u32 = 50;
// All Surface gesture timers share increasing identities, avoiding late WM_TIMER collisions across gesture types.
pub(super) fn allocate_id() -> Result<usize> {
    static NEXT: AtomicUsize = AtomicUsize::new(0x4000);
    NEXT.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |v| v.checked_add(1))
        .map_err(|_| Error::new(E_FAIL, "Surface gesture timer identity exhausted"))
}
pub(super) struct Repeat {
    pub node: String,
    pub axis: usize,
    pub direction: i8,
    pub point: [i32; 2],
    hwnd: HWND,
    id: usize,
    due: Instant,
    repeating: bool,
}
impl Repeat {
    pub fn new(
        hwnd: HWND,
        node: String,
        axis: usize,
        direction: i8,
        point: [i32; 2],
    ) -> Result<Self> {
        let id = allocate_id()?;
        // Set deadline before registration so tiny registration differences cannot delay an exactly due first message by a whole cycle.
        let due = Instant::now() + Duration::from_millis(DELAY_MS.into());
        if unsafe { SetTimer(Some(hwnd), id, DELAY_MS, None) } == 0 {
            return Err(Error::from_win32());
        }
        Ok(Self {
            node,
            axis,
            direction,
            point,
            hwnd,
            id,
            due,
            repeating: false,
        })
    }
    pub fn id(&self) -> usize {
        self.id
    }
    /// Only matching due messages advance one step; manual/stale/backlogged WM_TIMER messages cannot cause paging bursts.
    pub fn ready(&mut self, id: usize, now: Instant) -> Result<bool> {
        if id != self.id || now < self.due {
            return Ok(false);
        }
        if !self.repeating {
            if unsafe { SetTimer(Some(self.hwnd), self.id, INTERVAL_MS, None) } == 0 {
                return Err(Error::from_win32());
            }
            self.repeating = true;
        }
        self.due = now + Duration::from_millis(INTERVAL_MS.into());
        Ok(true)
    }
}
impl Drop for Repeat {
    fn drop(&mut self) {
        unsafe {
            let _ = KillTimer(Some(self.hwnd), self.id);
        }
    }
}

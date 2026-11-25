//! Draw native GDI surfaces offscreen before a single commit; the using window owner holds the thread-cached session.
use windows::{
    core::{Error, Result},
    Win32::{Foundation::RECT, Graphics::Gdi::HDC, UI::Controls::*},
};

pub(crate) struct Session;
impl Session {
    pub(crate) fn new() -> Result<Self> {
        unsafe { BufferedPaintInit()? };
        Ok(Self)
    }
}
impl Drop for Session {
    fn drop(&mut self) {
        if let Err(error) = unsafe { BufferedPaintUnInit() } {
            eprintln!("native-buffer-cleanup: {error}");
        }
    }
}

pub(crate) struct Frame {
    handle: isize,
    pub(crate) dc: HDC,
}
impl Frame {
    // rect comes from BeginPaint already constrained by the window pixel budget; do not cache a full-screen bitmap per node.
    pub(crate) unsafe fn begin(target: HDC, rect: &RECT) -> Result<Option<Self>> {
        if rect.right <= rect.left || rect.bottom <= rect.top {
            return Ok(None);
        }
        let mut dc = HDC::default();
        let handle = BeginBufferedPaint(target, rect, BPBF_COMPATIBLEBITMAP, None, &mut dc);
        if handle == 0 {
            return Err(Error::from_win32());
        }
        Ok(Some(Self { handle, dc }))
    }
    pub(crate) unsafe fn commit(mut self) -> Result<()> {
        let handle = std::mem::replace(&mut self.handle, 0);
        EndBufferedPaint(handle, true)
    }
}
impl Drop for Frame {
    fn drop(&mut self) {
        if self.handle != 0 {
            // Discard incomplete frames on panic/drawing failure; never publish a partially cleared overlay to the desktop.
            if let Err(error) = unsafe { EndBufferedPaint(self.handle, false) } {
                eprintln!("native-buffer-discard: {error}");
            }
        }
    }
}

#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-canvas")))]
#[path = "buffered_paint_tests.rs"]
mod tests;

//! Immutable software frames on the UI thread: own DIB/pixel budgets without borrowing Canvas, D2D targets, or window state.
use super::bitmap::Bitmap;
use std::rc::Rc;
use windows::{
    core::{Error, Result},
    Win32::{
        Foundation::*, Graphics::Gdi::*, System::Threading::GetCurrentThreadId,
        UI::WindowsAndMessaging::*,
    },
};

/// A single-consumption submission credential. Rc confines it to the UI thread and extends DC/DIB lifetime; no pixel copies or background queue.
pub struct Submission {
    bitmap: Rc<Bitmap>,
    pinned: bool,
    opacity: f32,
}
impl Submission {
    pub(in crate::desktop) fn framed(self, padding: u32, border: bool) -> Result<Self> {
        Ok(Self::new(
            Rc::new(self.bitmap.framed(padding, self.opacity, border)?),
            true,
            1.0,
        ))
    }
    pub(super) fn new(bitmap: Rc<Bitmap>, pinned: bool, opacity: f32) -> Self {
        Self {
            bitmap,
            pinned,
            opacity,
        }
    }
    /// Callers own the target HWND and call after releasing Canvas/State borrows; confirm presentation counts only on success.
    /// UpdateLayeredWindow may synchronously send size messages; this credential independently retains source pixels during reentrancy.
    /// Success and failure both consume the credential; do not retry or change window styles here.
    pub fn present(self, hwnd: HWND) -> Result<()> {
        self.present_at(hwnd, None)
    }
    pub(in crate::desktop) fn present_at(self, hwnd: HWND, position: Option<POINT>) -> Result<()> {
        unsafe {
            if !IsWindow(Some(hwnd)).as_bool()
                || GetWindowThreadProcessId(hwnd, None) != GetCurrentThreadId()
            {
                return Err(Error::new(
                    E_INVALIDARG,
                    "Software submission requires a UI-thread HWND",
                ));
            }
            let style = WINDOW_EX_STYLE(GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32);
            if (!self.pinned && style.contains(WS_EX_NOREDIRECTIONBITMAP))
                || style.contains(WS_EX_LAYERED) != self.pinned
            {
                return Err(Error::new(
                    E_INVALIDARG,
                    "Software HWND style is incompatible",
                ));
            }
            let size = self.bitmap.size;
            if self.pinned {
                // Without a new position, ULW updates in place without reading back window coordinates.
                UpdateLayeredWindow(
                    hwnd,
                    None,
                    position.as_ref().map(std::ptr::from_ref),
                    Some(&SIZE {
                        cx: size.width as i32,
                        cy: size.height as i32,
                    }),
                    Some(self.bitmap.dc()),
                    Some(&POINT::default()),
                    COLORREF(0),
                    Some(&BLENDFUNCTION {
                        BlendOp: AC_SRC_OVER as u8,
                        BlendFlags: 0,
                        SourceConstantAlpha: (self.opacity * 255.0).round() as u8,
                        AlphaFormat: AC_SRC_ALPHA as u8,
                    }),
                    ULW_ALPHA,
                )?;
                GdiFlush().ok()
            } else {
                let dc = GetDC(Some(hwnd));
                if dc.is_invalid() {
                    return Err(Error::new(E_FAIL, "Software window DC unavailable"));
                }
                let dc = WindowDc(hwnd, dc);
                let copied = BitBlt(
                    dc.1,
                    0,
                    0,
                    size.width as i32,
                    size.height as i32,
                    Some(self.bitmap.dc()),
                    0,
                    0,
                    SRCCOPY | NOMIRRORBITMAP,
                );
                // Flush only once before releasing the DC/source bitmap, preserving transfer or flush failures.
                let flushed = GdiFlush().ok();
                copied.and(flushed)
            }
        }
    }
}
pub(in crate::desktop) fn validate(pinned: bool, opacity: f32) -> Result<()> {
    if !opacity.is_finite() || !(0.0..=1.0).contains(&opacity) || (!pinned && opacity != 1.0) {
        return Err(Error::new(E_INVALIDARG, "Invalid software window opacity"));
    }
    Ok(())
}
struct WindowDc(HWND, HDC);
impl Drop for WindowDc {
    fn drop(&mut self) {
        unsafe {
            let _ = ReleaseDC(Some(self.0), self.1);
        }
    }
}

#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-canvas")))]
#[path = "software_submission_tests.rs"]
mod tests;

//! UI 线程不可变软件帧：持有 DIB/像素预算，不借 Canvas、D2D target 或窗口状态。
use super::bitmap::Bitmap;
use std::rc::Rc;
use windows::{
    core::{Error, Result},
    Win32::{
        Foundation::*, Graphics::Gdi::*, System::Threading::GetCurrentThreadId,
        UI::WindowsAndMessaging::*,
    },
};

/// 单次消费的提交凭证。Rc 限定 UI 线程并延长 DC/DIB 生命期；不复制像素、不建立后台队列。
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
    /// 调用方拥有目标 HWND，并应在释放 Canvas/State 借用后调用；成功才可确认呈现计数。
    /// UpdateLayeredWindow 可同步发送尺寸消息，本凭证在重入期间独立保持源像素。
    /// 成功或失败均消费凭证，不在这里重试或更改窗口样式。
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
                // 未指定新位置时由 ULW 原地更新，不读回窗口坐标。
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
                // 在释放 DC 和源位图前只刷新一次，并保留传输或刷新失败。
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

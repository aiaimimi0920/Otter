//! 四条窄顶层窗口标识采集边界；不覆盖整块桌面、不激活、不接收鼠标，关闭即销毁。
use super::*;
use windows::{
    core::w,
    Win32::{System::SystemServices::SS_WHITERECT, UI::HiDpi::GetDpiForWindow},
};
pub(super) struct Outline {
    windows: Vec<crate::desktop::Window>,
    visible: bool,
}
impl Outline {
    pub unsafe fn new(owner: HWND, region: Region, visible: bool, protect: bool) -> Result<Self> {
        let mut outline = Self {
            windows: Vec::with_capacity(4),
            visible,
        };
        for _ in 0..4 {
            let window = crate::desktop::Window(CreateWindowExW(
                WS_EX_TOOLWINDOW
                    | WS_EX_TOPMOST
                    | WS_EX_NOACTIVATE
                    | WS_EX_TRANSPARENT
                    | WS_EX_LAYERED,
                w!("STATIC"),
                w!(""),
                WS_POPUP | WS_DISABLED | WS_BORDER | WINDOW_STYLE(SS_WHITERECT.0),
                0,
                0,
                1,
                1,
                Some(owner),
                None,
                None,
                None,
            )?);
            // Layered + TRANSPARENT 才明确提供跨窗口鼠标穿透；不可用时不显示候选窗口。
            SetLayeredWindowAttributes(window.0, COLORREF(0), 255, LWA_ALPHA)?;
            if protect {
                SetWindowDisplayAffinity(window.0, WDA_EXCLUDEFROMCAPTURE)?;
            }
            outline.windows.push(window);
        }
        outline.position(region)?;
        Ok(outline)
    }
    pub unsafe fn position(&self, region: Region) -> Result<()> {
        let mut dpi = GetDpiForWindow(self.windows[0].0).max(96);
        // 跨显示器定位可能改变 DPI，最多校正一次；不安装布局计时器。
        for _ in 0..2 {
            let rects = rectangles(region, dpi)?;
            for (window, rect) in self.windows.iter().zip(rects) {
                let flags = SWP_NOACTIVATE
                    | if self.visible {
                        SWP_SHOWWINDOW
                    } else {
                        SWP_NOZORDER
                    };
                if let Err(error) = SetWindowPos(
                    window.0,
                    Some(HWND_TOPMOST),
                    rect.left,
                    rect.top,
                    rect.right - rect.left,
                    rect.bottom - rect.top,
                    flags,
                ) {
                    self.hide();
                    return Err(error);
                }
            }
            let next = GetDpiForWindow(self.windows[0].0).max(96);
            if next == dpi {
                break;
            }
            dpi = next;
        }
        Ok(())
    }
    unsafe fn hide(&self) {
        for window in &self.windows {
            let _ = ShowWindow(window.0, SW_HIDE);
        }
    }
}
fn rectangles(region: Region, dpi: u32) -> Result<[RECT; 4]> {
    if region.width <= 0 || region.height <= 0 {
        return Err(Error::new(E_INVALIDARG, "Invalid viewfinder rectangle"));
    }
    let thickness = ((u64::from(dpi) * 3 + 48) / 96).clamp(3, 12) as i64;
    let left = i64::from(region.left);
    let top = i64::from(region.top);
    let right = left + i64::from(region.width);
    let bottom = top + i64::from(region.height);
    let make = |l, t, r, b| -> Result<RECT> {
        let convert = |n| {
            i32::try_from(n)
                .map_err(|_| Error::new(E_INVALIDARG, "Viewfinder outline coordinate overflow"))
        };
        let rect = RECT {
            left: convert(l)?,
            top: convert(t)?,
            right: convert(r)?,
            bottom: convert(b)?,
        };
        rect.right
            .checked_sub(rect.left)
            .filter(|n| *n > 0)
            .ok_or_else(|| Error::new(E_INVALIDARG, "Viewfinder outline width overflow"))?;
        rect.bottom
            .checked_sub(rect.top)
            .filter(|n| *n > 0)
            .ok_or_else(|| Error::new(E_INVALIDARG, "Viewfinder outline height overflow"))?;
        Ok(rect)
    };
    Ok([
        make(left - thickness, top - thickness, right + thickness, top)?,
        make(
            left - thickness,
            bottom,
            right + thickness,
            bottom + thickness,
        )?,
        make(left - thickness, top, left, bottom)?,
        make(right, top, right + thickness, bottom)?,
    ])
}
#[cfg(test)]
#[path = "long_capture_outline_tests.rs"]
mod tests;

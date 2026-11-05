//! 接收原图的同步呈现围栏；只证明可见 HWND 的新帧已提交，不宣称物理屏幕扫描输出。
use super::*;
use crate::desktop::document_window::DocumentWindow;
use windows::Win32::Graphics::Gdi::{RedrawWindow, RDW_INVALIDATE, RDW_UPDATENOW};
use windows::Win32::Graphics::{
    Dwm::{DwmGetWindowAttribute, DWMWA_CLOAKED},
    Gdi::{GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONULL},
};

pub(super) struct Visual {
    hwnd: HWND,
    state: usize,
    document: restore_batch::guard::Document,
    image: crate::image_decode::Image,
}
impl Visual {
    pub(super) unsafe fn capture(doc: &DocumentWindow) -> Result<Self> {
        let value = Self {
            hwnd: doc.window.0,
            state: (&*doc.state as *const State) as usize,
            document: restore_batch::guard::Document::capture(doc)?,
            image: doc
                .state
                .canvas
                .borrow()
                .image
                .as_ref()
                .ok_or_else(stale)?
                .clone(),
        };
        if !value.matches(doc) {
            return Err(stale());
        }
        Ok(value)
    }
    pub(super) unsafe fn matches(&self, doc: &DocumentWindow) -> bool {
        self.mismatch(doc).is_none()
    }
    pub(super) unsafe fn mismatch(&self, doc: &DocumentWindow) -> Option<&'static str> {
        if let Some(reason) = self.document.mismatch(doc) {
            return Some(reason);
        }
        macro_rules! require {
            ($condition:expr, $reason:literal) => {
                if !$condition {
                    return Some($reason);
                }
            };
        }
        let canvas = doc.state.canvas.borrow();
        require!(doc.window.0 == self.hwnd, "visual-window");
        require!(
            (&*doc.state as *const State) as usize == self.state,
            "visual-state"
        );
        require!(doc.is_alive(), "visual-lifetime");
        require!(!doc.state.group_hidden.get(), "hidden");
        require!(
            doc.state.resolved_display.borrow().is_none(),
            "resolved-display"
        );
        require!(doc.state.error.borrow().is_none(), "render-error");
        require!(canvas.pinned, "unpinned");
        require!(canvas.minified.is_none(), "minified");
        require!(
            canvas.window_opacity.partial_cmp(&0.0) == Some(std::cmp::Ordering::Greater),
            "opacity"
        );
        require!(
            canvas.image.as_ref().is_some_and(|image| {
                image.width == self.image.width
                    && image.height == self.image.height
                    && image.pixels.same_storage(&self.image.pixels)
            }),
            "image-storage"
        );
        None
    }
    pub(super) unsafe fn visible(&self, doc: &DocumentWindow) -> bool {
        let mut rect = RECT::default();
        let mut frame = RECT::default();
        let mut cloaked = 0u32;
        let mut monitor = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        self.matches(doc)
            && IsWindowVisible(self.hwnd).as_bool()
            && !IsIconic(self.hwnd).as_bool()
            && GetClientRect(self.hwnd, &mut rect).is_ok()
            && rect.right > rect.left
            && rect.bottom > rect.top
            && DwmGetWindowAttribute(
                self.hwnd,
                DWMWA_CLOAKED,
                (&mut cloaked as *mut u32).cast(),
                4,
            )
            .is_ok()
            && cloaked == 0
            && GetWindowRect(self.hwnd, &mut frame).is_ok()
            && GetMonitorInfoW(
                MonitorFromWindow(self.hwnd, MONITOR_DEFAULTTONULL),
                &mut monitor,
            )
            .as_bool()
            && frame.right > monitor.rcWork.left
            && frame.left < monitor.rcWork.right
            && frame.bottom > monitor.rcWork.top
            && frame.top < monitor.rcWork.bottom
    }
    pub(super) unsafe fn present(&self, doc: &DocumentWindow) -> Result<()> {
        if !self.matches(doc) {
            return Err(Error::new(
                E_ABORT,
                format!(
                    "Projection visual changed before presentation: {:?}",
                    self.mismatch(doc)
                ),
            ));
        }
        doc.observe_priority()?;
        if !self.matches(doc) {
            return Err(Error::new(
                E_ABORT,
                format!(
                    "Projection visual changed while installing watcher: {:?}",
                    self.mismatch(doc)
                ),
            ));
        }
        // 不跨可重入 Win32 调用保留 RefCell 借用；再次检查同 HWND/State/文档/像素。
        let _ = ShowWindow(self.hwnd, SW_SHOWNOACTIVATE);
        if !self.visible(doc) {
            return Err(Error::new(
                E_ABORT,
                format!(
                    "Projection window is not visible after showing: {:?}",
                    self.mismatch(doc)
                ),
            ));
        }
        let before = {
            let canvas = doc.state.canvas.borrow();
            (canvas.paints, canvas.confirmed_paints)
        };
        if !RedrawWindow(Some(self.hwnd), None, None, RDW_INVALIDATE | RDW_UPDATENOW).as_bool() {
            return Err(Error::new(E_ABORT, "Projection synchronous redraw failed"));
        }
        if !self.visible(doc) {
            return Err(Error::new(
                E_ABORT,
                format!(
                    "Projection visual changed or became invisible during redraw: {:?}",
                    self.mismatch(doc)
                ),
            ));
        }
        let after = {
            let canvas = doc.state.canvas.borrow();
            (canvas.paints, canvas.confirmed_paints)
        };
        if after.1 <= before.1 {
            return Err(Error::new(
                E_ABORT,
                format!("Projection redraw did not confirm a new frame: paints/confirmed {before:?} -> {after:?}"),
            ));
        }
        Ok(())
    }
}

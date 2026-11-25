//! 原生 GDI 表面先离屏绘制再一次提交；线程缓存会话由使用它的窗口 owner 持有。
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
    // rect 来自已受窗口像素预算约束的 BeginPaint；不为每个节点缓存一张整屏位图。
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
            // panic/绘制失败丢弃未完成帧，绝不把清空到一半的叠层发布给桌面。
            if let Err(error) = unsafe { EndBufferedPaint(self.handle, false) } {
                eprintln!("native-buffer-discard: {error}");
            }
        }
    }
}

#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-canvas")))]
#[path = "buffered_paint_tests.rs"]
mod tests;

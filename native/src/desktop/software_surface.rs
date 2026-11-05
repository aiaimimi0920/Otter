//! 软件 D2D → GDI DIB → HWND/分层窗口；不创建 D3D/DXGI/DirectComposition 呈现资源。
use crate::software_dib as bitmap;
#[path = "software_submission.rs"]
mod submission;
use std::rc::Rc;
pub(super) use submission::validate as validate_presentation;
pub use submission::Submission;
#[path = "software_window_style.rs"]
pub mod window_style;
use windows::{
    core::{Error, Result},
    Win32::{
        Foundation::*,
        Graphics::{
            Direct2D::{Common::*, *},
            Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM,
            Gdi::*,
        },
    },
};

pub struct SoftwareSurface {
    // target 必须先于它绑定的 DC/DIB 析构。
    target: ID2D1DCRenderTarget,
    bitmap: Rc<bitmap::Bitmap>,
    // BindDC 失败后的绑定状态不作猜测；两个 DC 都保留到 target 释放，禁止悬空引用。
    _failed_bitmap: Option<Rc<bitmap::Bitmap>>,
    // 内部位图按两份历史最大像素预留，缩小时不假定 D2D 已释放缓存；对象/驱动开销仍须实测。
    scratch: crate::byte_budget::Lease,
    ready: bool,
    failed: bool,
}
impl SoftwareSurface {
    #[cfg(test)]
    pub(in crate::desktop) fn readback_for_test(
        &self,
    ) -> crate::desktop::presentation_readback_test::Pixels {
        assert!(self.bitmap.size.width <= 256 && self.bitmap.size.height <= 256);
        let length = self.bitmap.size.width as usize * self.bitmap.size.height as usize * 4;
        let lease = crate::pixels::reserve(length * 2).unwrap();
        let bytes = self.bitmap.pixels().into_iter().flatten().collect();
        crate::desktop::presentation_readback_test::Pixels {
            size: [self.bitmap.size.width, self.bitmap.size.height],
            bytes: crate::pixels::Pixels::from_reserved(bytes, lease),
        }
    }
    pub fn new(factory: &ID2D1Factory, size: D2D_SIZE_U) -> Result<Self> {
        let bitmap = Rc::new(bitmap::Bitmap::new(size)?);
        let scratch = crate::pixels::reserve(bitmap::byte_count(size)? * 2)
            .ok_or_else(|| Error::new(E_OUTOFMEMORY, "Software scratch pixel budget exhausted"))?;
        unsafe {
            let target = factory.CreateDCRenderTarget(&D2D1_RENDER_TARGET_PROPERTIES {
                r#type: D2D1_RENDER_TARGET_TYPE_SOFTWARE,
                pixelFormat: D2D1_PIXEL_FORMAT {
                    format: DXGI_FORMAT_B8G8R8A8_UNORM,
                    alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
                },
                dpiX: 96.0,
                dpiY: 96.0,
                ..Default::default()
            })?;
            target.BindDC(bitmap.dc(), &rect(size))?;
            target.SetTextAntialiasMode(D2D1_TEXT_ANTIALIAS_MODE_GRAYSCALE);
            Ok(Self {
                target,
                bitmap,
                _failed_bitmap: None,
                scratch,
                ready: false,
                failed: false,
            })
        }
    }
    // 仅内部 Canvas 同步资源适配；不得保存 target 或绕过 draw 自行开启帧。
    pub(in crate::desktop) fn resource_target(&self) -> &ID2D1RenderTarget {
        &self.target
    }
    pub fn resize(&mut self, size: D2D_SIZE_U) -> Result<()> {
        if self.failed {
            return Err(Error::new(E_FAIL, "Software surface requires rebuild"));
        }
        if size == self.bitmap.size {
            return Ok(());
        }
        self.ready = false;
        let next = Rc::new(bitmap::Bitmap::new(size)?);
        if !self.scratch.grow_to(bitmap::byte_count(size)? * 2) {
            return Err(Error::new(
                E_OUTOFMEMORY,
                "Software scratch growth exceeds pixel budget",
            ));
        }
        if let Err(error) = unsafe { self.target.BindDC(next.dc(), &rect(size)) } {
            self.failed = true;
            self._failed_bitmap = Some(next);
            return Err(error);
        }
        self.bitmap = next;
        Ok(())
    }
    /// 内部可信绘制适配：闭包不得保存/克隆 target 到作用域外，也不得自行 BeginDraw/EndDraw。
    /// 租约保护覆盖本 owner 的绘制入口；不是对绕过接口的任意 unsafe COM 操作的隔离。
    pub fn draw(&mut self, paint: impl FnOnce(&ID2D1RenderTarget) -> Result<()>) -> Result<()> {
        self.ready = false;
        if self.failed {
            return Err(Error::new(E_FAIL, "Software surface requires rebuild"));
        }
        // 未提交的帧拥有同一 DIB 时，不允许后续绘制覆盖其像素；取消/提交后自动恢复。
        if Rc::strong_count(&self.bitmap) != 1 {
            return Err(Error::new(
                E_FAIL,
                "Software frame is still pending submission",
            ));
        }
        super::render_frame::draw(&self.target, || paint(&self.target))?;
        // EndDraw 将内部位图送到 GDI；读取 DIB/提交窗口前排空本线程的 GDI 批次。
        unsafe {
            GdiFlush().ok()?;
        }
        self.ready = true;
        Ok(())
    }
    /// 调用者拥有 HWND 样式；本候选不擅自切换窗口的重定向/分层标记。
    pub fn present(&mut self, hwnd: HWND, pinned: bool, opacity: f32) -> Result<()> {
        self.prepare_present(pinned, opacity)?.present(hwnd)
    }
    /// 只转移不可变像素租约，不调用 HWND API；窗口 owner 应释放模型借用后再提交。
    /// 此后同一 DIB 禁止绘制，直到提交或丢弃；resize 的新 DIB 仍受共享预算约束。
    pub fn prepare_present(&mut self, pinned: bool, opacity: f32) -> Result<Submission> {
        submission::validate(pinned, opacity)?;
        if !std::mem::take(&mut self.ready) {
            return Err(Error::new(
                E_FAIL,
                "Software present requires completed draw",
            ));
        }
        Ok(Submission::new(Rc::clone(&self.bitmap), pinned, opacity))
    }
}
fn rect(size: D2D_SIZE_U) -> RECT {
    RECT {
        left: 0,
        top: 0,
        right: size.width as i32,
        bottom: size.height as i32,
    }
}
#[cfg(test)]
#[path = "software_surface_tests.rs"]
mod tests;

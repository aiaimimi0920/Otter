//! Software D2D -> GDI DIB -> HWND/layered window; create no D3D/DXGI/DirectComposition presentation resources.
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
    // Destroy target before its bound DC/DIB.
    target: ID2D1DCRenderTarget,
    bitmap: Rc<bitmap::Bitmap>,
    // Do not guess binding state after BindDC failure; retain both DCs until target releases, preventing dangling references.
    _failed_bitmap: Option<Rc<bitmap::Bitmap>>,
    // Reserve internal bitmaps for twice the historical maximum pixels; shrinking does not assume D2D released caches. Object/driver overhead still needs measurement.
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
    // Only internal Canvas synchronous-resource adaptation; never retain target or start frames outside draw.
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
    /// Trusted internal drawing adapter: closures must neither retain/clone target outside the scope nor call BeginDraw/EndDraw themselves.
    /// Lease protection covers this owner's drawing entry, not arbitrary unsafe COM operations bypassing the interface.
    pub fn draw(&mut self, paint: impl FnOnce(&ID2D1RenderTarget) -> Result<()>) -> Result<()> {
        self.ready = false;
        if self.failed {
            return Err(Error::new(E_FAIL, "Software surface requires rebuild"));
        }
        // If an unsubmitted frame owns the same DIB, later drawing cannot overwrite its pixels; cancellation/submission restores access automatically.
        if Rc::strong_count(&self.bitmap) != 1 {
            return Err(Error::new(
                E_FAIL,
                "Software frame is still pending submission",
            ));
        }
        super::render_frame::draw(&self.target, || paint(&self.target))?;
        // EndDraw transfers the internal bitmap to GDI; drain this thread's GDI batch before reading the DIB/submitting the window.
        unsafe {
            GdiFlush().ok()?;
        }
        self.ready = true;
        Ok(())
    }
    /// Callers own HWND styles; this candidate never changes window redirection/layered flags autonomously.
    pub fn present(&mut self, hwnd: HWND, pinned: bool, opacity: f32) -> Result<()> {
        self.prepare_present(pinned, opacity)?.present(hwnd)
    }
    /// Transfer only an immutable pixel lease without HWND API calls; window owners submit after releasing model borrows.
    /// Do not draw into the same DIB until submission/discard; newly resized DIBs remain subject to the shared budget.
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

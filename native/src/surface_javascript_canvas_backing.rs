//! Canvas 的线程绑定软件 D2D/DIB 背板；复用软件呈现的叶资源，不创建窗口或 GPU 呈现图。
use crate::{
    byte_budget,
    software_dib::Bitmap,
    surface_javascript_wire::{canvas, invalid},
};
use std::{io, sync::Arc};
use windows::Win32::{
    Foundation::RECT,
    Graphics::{
        Direct2D::{Common::*, *},
        Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM,
        Gdi::GdiFlush,
    },
};

pub(super) struct Backing {
    // target 必须先于其 BindDC 指向的 DIB/DC 释放；不向外交付可保留的 COM 克隆。
    pub(super) target: ID2D1DCRenderTarget,
    bitmap: Bitmap,
    _logical: byte_budget::Lease,
    // 和软件呈现一致，为 D2D 内部中间位图预留两份；Job 仍是整 child 的硬界。
    _scratch: byte_budget::Lease,
}
impl Backing {
    pub(super) fn new(
        factory: &ID2D1Factory,
        size: [u32; 2],
        budget: &Arc<byte_budget::Budget>,
        #[cfg(test)] trace: &mut crate::surface_javascript_wire::diagnostics::Trace,
    ) -> io::Result<Self> {
        #[cfg(test)]
        trace.mark(crate::surface_javascript_wire::diagnostics::Stage::CanvasBacking);
        let count = canvas::bytes(size)?;
        let logical = budget.reserve(count).ok_or_else(invalid)?;
        let scratch = crate::pixels::reserve(count.checked_mul(2).ok_or_else(invalid)?)
            .ok_or_else(invalid)?;
        let bitmap = Bitmap::new(D2D_SIZE_U {
            width: size[0],
            height: size[1],
        })
        .map_err(|_| invalid())?;
        #[cfg(test)]
        trace.mark(crate::surface_javascript_wire::diagnostics::Stage::CanvasBitmapReady);
        let target = unsafe {
            let target = factory
                .CreateDCRenderTarget(&D2D1_RENDER_TARGET_PROPERTIES {
                    r#type: D2D1_RENDER_TARGET_TYPE_SOFTWARE,
                    dpiX: 96.,
                    dpiY: 96.,
                    pixelFormat: D2D1_PIXEL_FORMAT {
                        format: DXGI_FORMAT_B8G8R8A8_UNORM,
                        alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
                    },
                    ..Default::default()
                })
                .map_err(|_| invalid())?;
            #[cfg(test)]
            trace.mark(crate::surface_javascript_wire::diagnostics::Stage::CanvasTargetReady);
            target
                .BindDC(
                    bitmap.dc(),
                    &RECT {
                        left: 0,
                        top: 0,
                        right: size[0] as i32,
                        bottom: size[1] as i32,
                    },
                )
                .map_err(|_| invalid())?;
            #[cfg(test)]
            trace.mark(crate::surface_javascript_wire::diagnostics::Stage::CanvasBindReady);
            target.SetTextAntialiasMode(D2D1_TEXT_ANTIALIAS_MODE_GRAYSCALE);
            target.BeginDraw();
            target.Clear(Some(&D2D1_COLOR_F::default()));
            #[cfg(test)]
            trace.mark(crate::surface_javascript_wire::diagnostics::Stage::CanvasInitialEndDraw);
            target.EndDraw(None, None).map_err(|_| invalid())?;
            GdiFlush().ok().map_err(|_| invalid())?;
            target
        };
        #[cfg(test)]
        trace.mark(crate::surface_javascript_wire::diagnostics::Stage::CanvasBackingReady);
        Ok(Self {
            target,
            bitmap,
            _logical: logical,
            _scratch: scratch,
        })
    }
    pub(super) fn copy_into(&self, output: &mut [u8]) -> io::Result<()> {
        self.bitmap.copy_into(output).map_err(|_| invalid())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn canvas_dib_backing_preserves_alpha_exact_copy_bounds_and_budget_release() {
        let factory: ID2D1Factory =
            unsafe { D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None).unwrap() };
        let budget = byte_budget::Budget::new(8);
        let mut trace = crate::surface_javascript_wire::diagnostics::Trace::default();
        let backing = Backing::new(&factory, [2, 1], &budget, &mut trace).unwrap();
        use crate::surface_javascript_wire::diagnostics::Stage;
        let stages: Vec<_> = trace
            .samples()
            .into_iter()
            .map(|sample| sample.stage)
            .collect();
        assert_eq!(
            stages,
            [
                Stage::CanvasBacking,
                Stage::CanvasBitmapReady,
                Stage::CanvasTargetReady,
                Stage::CanvasBindReady,
                Stage::CanvasInitialEndDraw,
                Stage::CanvasBackingReady
            ]
        );
        assert_eq!(budget.used(), 8);
        let mut pixels = [1; 8];
        backing.copy_into(&mut pixels).unwrap();
        assert_eq!(pixels, [0; 8]);
        let mut too_short = [99; 7];
        assert!(backing.copy_into(&mut too_short).is_err());
        assert_eq!(too_short, [99; 7]);
        unsafe {
            backing.target.BeginDraw();
            let brush = backing
                .target
                .CreateSolidColorBrush(
                    &D2D1_COLOR_F {
                        r: 1.,
                        g: 0.,
                        b: 0.,
                        a: 0.5,
                    },
                    None,
                )
                .unwrap();
            backing.target.FillRectangle(
                &D2D_RECT_F {
                    left: 0.,
                    top: 0.,
                    right: 1.,
                    bottom: 1.,
                },
                &brush,
            );
            backing.target.EndDraw(None, None).unwrap();
        }
        backing.copy_into(&mut pixels).unwrap();
        assert_eq!(&pixels[..2], &[0, 0]);
        assert!(pixels[2].abs_diff(128) <= 1 && pixels[3].abs_diff(128) <= 1);
        assert_eq!(&pixels[4..], &[0; 4]);
        assert!(Backing::new(&factory, [1, 1], &budget, &mut trace).is_err());
        drop(backing);
        assert_eq!(budget.used(), 0);
        drop(Backing::new(&factory, [2, 1], &budget, &mut trace).unwrap());
        assert_eq!(budget.used(), 0);
    }
}

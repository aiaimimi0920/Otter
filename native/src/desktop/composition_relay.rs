//! 远程 PBGRA 帧的窗口私有呈现；只在 UI 线程上传最新帧，不复制 CPU 像素缓存。
use super::*;
use crate::{live_relay_frame::Metadata, live_relay_pixels::PixelFrame};

pub(super) struct Surface {
    composition: Composition,
    bitmap: ID2D1Bitmap,
    metadata: Metadata,
    presented: Option<[u32; 2]>,
}
impl Surface {
    fn new(hwnd: HWND, size: D2D_SIZE_U, frame: &PixelFrame) -> Result<Self> {
        let composition = Composition::new(hwnd, size)?;
        let bitmap = bitmap(&composition, frame)?;
        Ok(Self {
            composition,
            bitmap,
            metadata: frame.metadata(),
            presented: None,
        })
    }
    fn set_frame(&mut self, frame: &PixelFrame) -> Result<()> {
        self.presented = None;
        let metadata = frame.metadata();
        if (metadata.width, metadata.height) == (self.metadata.width, self.metadata.height) {
            unsafe {
                self.bitmap.CopyFromMemory(
                    None,
                    frame.pixels().as_ptr().cast(),
                    metadata.width * 4,
                )?;
            }
        } else {
            self.bitmap = bitmap(&self.composition, frame)?;
        }
        self.metadata = metadata;
        Ok(())
    }
    fn draw(&mut self, size: D2D_SIZE_U) -> Result<()> {
        self.composition.resize(size)?;
        let viewport = crate::live_viewport::Viewport::fit(
            [self.metadata.width, self.metadata.height],
            [size.width, size.height],
        )
        .ok_or_else(|| Error::new(E_FAIL, "Invalid remote viewport"))?;
        let [left, top, right, bottom] = viewport.destination();
        unsafe {
            let target = self.composition.renderer();
            target.BeginDraw();
            target.Clear(Some(&D2D1_COLOR_F::default()));
            target.DrawBitmap(
                &self.bitmap,
                Some(&D2D_RECT_F {
                    left,
                    top,
                    right,
                    bottom,
                }),
                1.0,
                D2D1_BITMAP_INTERPOLATION_MODE_LINEAR,
                None,
            );
            target.EndDraw(None, None)?;
        }
        Ok(())
    }
}
fn bitmap(composition: &Composition, frame: &PixelFrame) -> Result<ID2D1Bitmap> {
    let metadata = frame.metadata();
    unsafe {
        composition.renderer().CreateBitmap(
            D2D_SIZE_U {
                width: metadata.width,
                height: metadata.height,
            },
            Some(frame.pixels().as_ptr().cast()),
            metadata.width * 4,
            &D2D1_BITMAP_PROPERTIES {
                pixelFormat: D2D1_PIXEL_FORMAT {
                    format: DXGI_FORMAT_B8G8R8A8_UNORM,
                    alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
                },
                dpiX: 96.0,
                dpiY: 96.0,
            },
        )
    }
}

#[path = "composition_relay_host.rs"]
mod host;
pub use host::{Host, Update};
#[path = "composition_relay_session.rs"]
pub mod session;
#[path = "composition_relay_sessions.rs"]
pub mod sessions;
#[cfg(test)]
#[path = "composition_relay_tests.rs"]
mod tests;
#[path = "composition_relay_window.rs"]
pub mod window;

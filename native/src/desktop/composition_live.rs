//! 同设备 Live 纹理呈现；位图和完整帧 owner 一起保留，重复绘制不重建位图或复制像素。
use super::*;
use crate::{live_frames::Frame, live_texture::Texture};
use std::sync::Arc;
use windows::core::IUnknown;
#[path = "composition_live_chrome.rs"]
mod chrome;
#[path = "composition_live_host.rs"]
pub mod host;
#[path = "composition_live_notify.rs"]
pub mod notification;
#[path = "composition_live_session.rs"]
pub mod session;
#[path = "composition_live_window.rs"]
pub mod window;
pub struct Surface {
    composition: Composition,
    bitmap: ID2D1Bitmap1,
    frame: Arc<Frame<Texture>>,
    device: IUnknown,
    chrome: chrome::Chrome,
}
impl Surface {
    pub fn new(hwnd: HWND, size: D2D_SIZE_U, frame: Arc<Frame<Texture>>) -> Result<Self> {
        let device = validate(&frame.resource)?;
        let composition = Composition::with_device(hwnd, size, &device)?;
        let bitmap = bitmap(&composition, &frame.resource)?;
        let chrome = chrome::Chrome::new(composition.renderer())?;
        Ok(Self {
            composition,
            bitmap,
            frame,
            device: device.cast()?,
            chrome,
        })
    }
    /// 更换设备必须由宿主重建整个 surface；失败不改动当前帧和位图。
    pub fn set_frame(&mut self, frame: Arc<Frame<Texture>>) -> Result<bool> {
        if Arc::ptr_eq(&frame, &self.frame) {
            return Ok(false);
        }
        let device = validate(&frame.resource)?;
        if device.cast::<IUnknown>()? != self.device {
            return Err(invalid("Live frame belongs to another device"));
        }
        let candidate = bitmap(&self.composition, &frame.resource)?;
        self.bitmap = candidate;
        self.frame = frame;
        Ok(true)
    }
    pub fn resize(&mut self, size: D2D_SIZE_U) -> Result<()> {
        self.composition.resize(size)
    }
    pub fn set_opacity(&mut self, opacity: f32) -> Result<()> {
        self.composition.set_opacity(opacity)
    }
    pub fn draw(&self) -> Result<()> {
        self.draw_chrome(true)
    }
    pub(in crate::desktop) fn draw_chrome(&self, visible: bool) -> Result<()> {
        self.composition.ensure_attached()?;
        let size = self.composition.state.size;
        let viewport = crate::live_viewport::Viewport::fit(
            [self.frame.resource.width, self.frame.resource.height],
            [size.width, size.height],
        )
        .ok_or_else(|| invalid("Invalid Live viewport geometry"))?;
        let [left, top, right, bottom] = viewport.destination();
        let destination = D2D_RECT_F {
            left,
            top,
            right,
            bottom,
        };
        unsafe {
            let target = self.composition.renderer();
            target.BeginDraw();
            target.Clear(Some(&D2D1_COLOR_F::default()));
            target.DrawBitmap(
                &self.bitmap,
                Some(&destination),
                1.0,
                D2D1_BITMAP_INTERPOLATION_MODE_LINEAR,
                None,
            );
            if visible {
                self.chrome.draw(target, size);
            }
            target.EndDraw(None, None)
        }
    }
    pub fn present(&self) -> Result<()> {
        self.composition.present()
    }
}
fn invalid(message: &str) -> Error {
    Error::new(windows::Win32::Foundation::E_INVALIDARG, message)
}
fn validate(frame: &Texture) -> Result<ID3D11Device> {
    unsafe {
        let mut desc = D3D11_TEXTURE2D_DESC::default();
        frame.texture().GetDesc(&mut desc);
        if desc.Format != DXGI_FORMAT_B8G8R8A8_UNORM
            || frame.format != desc.Format
            || frame.width != desc.Width
            || frame.height != desc.Height
            || desc.Width == 0
            || desc.Height == 0
        {
            return Err(invalid(
                "Live presentation requires matching BGRA8 texture metadata",
            ));
        }
        let device = frame.texture().GetDevice()?;
        let protected: ID3D11Multithread = device.GetImmediateContext()?.cast()?;
        if !protected.GetMultithreadProtected().as_bool() {
            return Err(invalid("Live presentation requires protected D3D context"));
        }
        Ok(device)
    }
}
fn bitmap(composition: &Composition, frame: &Texture) -> Result<ID2D1Bitmap1> {
    unsafe {
        // 生产捕获设备已启用 multithread protection；先提交同设备 immediate context 中的复制命令。
        frame.texture().GetDevice()?.GetImmediateContext()?.Flush();
        let surface: IDXGISurface = frame.texture().cast()?;
        composition.state.context.CreateBitmapFromDxgiSurface(
            &surface,
            Some(&D2D1_BITMAP_PROPERTIES1 {
                pixelFormat: D2D1_PIXEL_FORMAT {
                    format: DXGI_FORMAT_B8G8R8A8_UNORM,
                    alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
                },
                dpiX: 96.0,
                dpiY: 96.0,
                bitmapOptions: D2D1_BITMAP_OPTIONS_NONE,
                ..Default::default()
            }),
        )
    }
}
#[cfg(test)]
#[path = "composition_live_tests.rs"]
mod tests;

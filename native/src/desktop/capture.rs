//! 一次性 GDI 截图；不常驻采集，不读取剪贴板，不创建文件。
use super::image::Image;
#[path = "capture_dynamic_source.rs"]
mod dynamic;
pub use dynamic::region as dynamic_region;
#[path = "capture_wait_dialog.rs"]
mod wait_dialog;
pub use wait_dialog::show as wait_dynamic;
pub use wait_dialog::{show_cached as wait_cached, CachedRequest};
use windows::{
    core::{Error, Result},
    Win32::{Foundation::*, Graphics::Gdi::*, UI::WindowsAndMessaging::*},
};

struct Screen(HDC);
impl Drop for Screen {
    fn drop(&mut self) {
        unsafe {
            ReleaseDC(None, self.0);
        }
    }
}
struct MemoryDc(HDC);
impl Drop for MemoryDc {
    fn drop(&mut self) {
        unsafe {
            let _ = DeleteDC(self.0);
        }
    }
}
struct Bitmap(HBITMAP);
impl Drop for Bitmap {
    fn drop(&mut self) {
        unsafe {
            let _ = DeleteObject(self.0.into());
        }
    }
}
struct Selection {
    dc: HDC,
    previous: HGDIOBJ,
}
impl Drop for Selection {
    fn drop(&mut self) {
        unsafe {
            SelectObject(self.dc, self.previous);
        }
    }
}

fn dimensions(left: i32, top: i32, width: i32, height: i32) -> Result<usize> {
    if width <= 0
        || height <= 0
        || left.checked_add(width).is_none()
        || top.checked_add(height).is_none()
    {
        return Err(Error::new(E_INVALIDARG, "Invalid capture rectangle"));
    }
    crate::viewport::checked_image_bytes(width as u32, height as u32)
        .ok_or_else(|| Error::new(E_INVALIDARG, "Capture exceeds image budget"))
}

pub(super) struct Frame {
    pub image: Image,
    pub origin: POINT,
}
pub(super) fn virtual_desktop() -> Result<Frame> {
    unsafe {
        let left = GetSystemMetrics(SM_XVIRTUALSCREEN);
        let top = GetSystemMetrics(SM_YVIRTUALSCREEN);
        let width = GetSystemMetrics(SM_CXVIRTUALSCREEN);
        let height = GetSystemMetrics(SM_CYVIRTUALSCREEN);
        let image = region(left, top, width, height)?;
        Ok(Frame {
            image,
            origin: POINT { x: left, y: top },
        })
    }
}
pub(super) fn validate_region(left: i32, top: i32, width: i32, height: i32) -> Result<()> {
    dimensions(left, top, width, height).map(|_| ())
}
/// 物理屏幕坐标；每次只创建选区大小的 DIB，不先分配整屏截图。
pub(super) fn region(left: i32, top: i32, width: i32, height: i32) -> Result<Image> {
    dimensions(left, top, width, height)?;
    unsafe {
        let screen = Screen(GetDC(None));
        if screen.0.is_invalid() {
            return Err(Error::from_win32());
        }
        capture_with(screen.0, width, height, |destination| {
            BitBlt(
                destination,
                0,
                0,
                width,
                height,
                Some(screen.0),
                left,
                top,
                SRCCOPY | CAPTUREBLT,
            )
        })
    }
}

unsafe fn capture_with(
    base: HDC,
    width: i32,
    height: i32,
    fill: impl FnOnce(HDC) -> Result<()>,
) -> Result<Image> {
    let count = dimensions(0, 0, width, height)?;
    // DIB 与最终共享像素同时存在，分别持有预算，避免峰值只计算一份。
    let _dib_lease = crate::pixels::reserve(count)
        .ok_or_else(|| Error::new(E_OUTOFMEMORY, "Capture bitmap budget exhausted"))?;
    let mut lease = crate::pixels::reserve(count)
        .ok_or_else(|| Error::new(E_OUTOFMEMORY, "Capture pixel budget exhausted"))?;
    let dc = MemoryDc(CreateCompatibleDC(Some(base)));
    if dc.0.is_invalid() {
        return Err(Error::from_win32());
    }
    let info = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: width,
            biHeight: -height,
            biPlanes: 1,
            biBitCount: 32,
            ..Default::default()
        },
        ..Default::default()
    };
    let mut data = std::ptr::null_mut();
    let bitmap = Bitmap(CreateDIBSection(
        Some(dc.0),
        &info,
        DIB_RGB_COLORS,
        &mut data,
        None,
        0,
    )?);
    if bitmap.0.is_invalid() || data.is_null() {
        return Err(Error::new(E_FAIL, "Capture DIB unavailable"));
    }
    let previous = SelectObject(dc.0, bitmap.0.into());
    if previous.is_invalid() {
        return Err(Error::new(E_FAIL, "Cannot select capture bitmap"));
    }
    let _selection = Selection { dc: dc.0, previous };
    // 覆盖虚拟桌面中没有显示器的间隙；失败不得返回未初始化内容。
    std::ptr::write_bytes(data.cast::<u8>(), 0, count);
    fill(dc.0)?;
    if !GdiFlush().as_bool() {
        return Err(Error::from_win32());
    }
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(count)
        .map_err(|_| Error::new(E_OUTOFMEMORY, "Cannot allocate capture pixels"))?;
    if !lease.grow_to(bytes.capacity()) {
        return Err(Error::new(E_OUTOFMEMORY, "Capture capacity exceeds budget"));
    }
    bytes.extend_from_slice(std::slice::from_raw_parts(data.cast::<u8>(), count));
    // GDI 不保证 alpha，桌面截图为不透明 BGRA，因而也符合 PBGRA 格式。
    for pixel in bytes.chunks_exact_mut(4) {
        pixel[3] = 255;
    }
    Ok(Image {
        width: width as u32,
        height: height as u32,
        pixels: crate::pixels::Pixels::from_reserved(bytes, lease),
    })
}

#[cfg(test)]
pub(super) fn synthetic(
    width: i32,
    height: i32,
    fill: impl FnOnce(HDC) -> Result<()>,
) -> Result<Image> {
    unsafe {
        let base = MemoryDc(CreateCompatibleDC(None));
        if base.0.is_invalid() {
            return Err(Error::from_win32());
        }
        capture_with(base.0, width, height, fill)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn capture_validates_signed_coordinates_and_allocation_bounds() {
        assert_eq!(
            dimensions(-1920, -100, 3840, 1080).unwrap(),
            3840 * 1080 * 4
        );
        for rect in [
            (0, 0, 0, 1),
            (0, 0, 1, -1),
            (i32::MAX, 0, 2, 2),
            (0, 0, 20000, 20000),
        ] {
            assert!(dimensions(rect.0, rect.1, rect.2, rect.3).is_err());
        }
    }
    #[test]
    fn synthetic_capture_has_top_down_bgra_and_opaque_alpha() {
        unsafe {
            // 仅内存 DC；不调用 GetDC(None)，不读取用户屏幕。
            let base = MemoryDc(CreateCompatibleDC(None));
            let image = capture_with(base.0, 2, 2, |dc| {
                SetPixelV(dc, 0, 0, COLORREF(0x000000ff)).ok()?;
                SetPixelV(dc, 1, 1, COLORREF(0x00ff0000)).ok()
            })
            .unwrap();
            assert_eq!(&image.pixels[0..4], &[0, 0, 255, 255]);
            assert_eq!(&image.pixels[4..8], &[0, 0, 0, 255]);
            assert_eq!(&image.pixels[12..16], &[255, 0, 0, 255]);
            assert!(capture_with(base.0, 2, 2, |_| Err(Error::new(
                E_FAIL,
                "injected fill failure"
            )))
            .is_err());
        }
    }
}

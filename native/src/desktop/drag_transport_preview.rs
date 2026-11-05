//! Shell 拖动图像边界；只复制有界缩略图，不在 UI 线程解码 PNG 或访问文件。
use windows::{
    core::{Error, Result},
    Win32::{Foundation::*, Graphics::Gdi::*, System::Com::*, UI::Shell::*},
};
struct Bitmap(HBITMAP);
impl Drop for Bitmap {
    fn drop(&mut self) {
        unsafe {
            let _ = DeleteObject(self.0.into());
        }
    }
}
pub(super) fn attach(
    object: &IDataObject,
    preview: &crate::desktop::drag_preview::Preview,
) -> Result<()> {
    if preview.width == 0
        || preview.height == 0
        || preview.width > crate::desktop::drag_preview::MAX_EDGE
        || preview.height > crate::desktop::drag_preview::MAX_EDGE
        || preview.pixels.len() != (preview.width * preview.height * 4) as usize
    {
        return Err(Error::new(E_INVALIDARG, "Invalid Shell drag preview"));
    }
    unsafe {
        let info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: preview.width as i32,
                biHeight: -(preview.height as i32),
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut bits = std::ptr::null_mut();
        let bitmap = Bitmap(CreateDIBSection(
            None,
            &info,
            DIB_RGB_COLORS,
            &mut bits,
            None,
            0,
        )?);
        if bits.is_null() {
            return Err(Error::from_win32());
        }
        std::ptr::copy_nonoverlapping(preview.pixels.as_ptr(), bits.cast(), preview.pixels.len());
        let helper: IDragSourceHelper =
            CoCreateInstance(&CLSID_DragDropHelper, None, CLSCTX_INPROC_SERVER)?;
        let image = SHDRAGIMAGE {
            sizeDragImage: SIZE {
                cx: preview.width as i32,
                cy: preview.height as i32,
            },
            ptOffset: POINT {
                x: (preview.width / 2) as i32,
                y: (preview.height / 2) as i32,
            },
            hbmpDragImage: bitmap.0,
            crColorKey: COLORREF(0xffff_ffff),
        };
        helper.InitializeFromBitmap(&image, object)?;
        // 成功后由 Shell 图像管理器接管；失败路径仍由本地 RAII 回收。
        std::mem::forget(bitmap);
        Ok(())
    }
}

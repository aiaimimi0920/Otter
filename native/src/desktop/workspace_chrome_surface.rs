//! After fully drawing shared line layers, ULW commits position/size/color-key pixels together; never move old borders first.
use super::super::{window::MAX_PIXELS, Rect};
use windows::{
    core::{Error, Result},
    Win32::{Foundation::*, Graphics::Gdi::*, UI::WindowsAndMessaging::*},
};

pub(in crate::desktop::application::chrome) struct Surface {
    dc: HDC,
    bitmap: HBITMAP,
    old: HGDIOBJ,
    size: SIZE,
    _lease: crate::byte_budget::Lease,
    pub(super) content: Option<super::Content>,
    #[cfg(test)]
    pub(in crate::desktop::application::chrome) submissions: std::cell::Cell<u32>,
}
impl Surface {
    pub(in crate::desktop::application::chrome) fn fits(&self, size: SIZE) -> bool {
        self.size == size
    }
    #[cfg(test)]
    pub(in crate::desktop::application::chrome) fn size(&self) -> SIZE {
        self.size
    }
    pub(in crate::desktop::application::chrome) fn dc(&self) -> HDC {
        self.dc
    }
    pub(in crate::desktop::application::chrome) unsafe fn new(size: SIZE) -> Result<Self> {
        let count = i64::from(size.cx) * i64::from(size.cy);
        if size.cx <= 0 || size.cy <= 0 || size.cx > 32768 || size.cy > 32768 || count > MAX_PIXELS
        {
            return Err(Error::from(E_OUTOFMEMORY));
        }
        let lease = crate::pixels::reserve(count as usize * 4)
            .ok_or_else(|| Error::new(E_OUTOFMEMORY, "Workspace chrome pixel budget exhausted"))?;
        let dc = CreateCompatibleDC(None);
        if dc.is_invalid() {
            return Err(Error::from_win32());
        }
        let mut surface = Self {
            dc,
            bitmap: HBITMAP::default(),
            old: HGDIOBJ::default(),
            size,
            _lease: lease,
            content: None,
            #[cfg(test)]
            submissions: Default::default(),
        };
        let info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: size.cx,
                biHeight: -size.cy,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut bits = std::ptr::null_mut();
        surface.bitmap = CreateDIBSection(None, &info, DIB_RGB_COLORS, &mut bits, None, 0)?;
        surface.old = SelectObject(dc, surface.bitmap.into());
        if surface.old.is_invalid() {
            return Err(Error::from_win32());
        }
        Ok(surface)
    }
    pub(in crate::desktop::application::chrome) unsafe fn present(
        &self,
        hwnd: HWND,
        rect: Rect,
    ) -> Result<()> {
        #[cfg(test)]
        self.submissions.set(self.submissions.get() + 1);
        // Finish the GDI batch first; DWM never sees cleared partial frames or old-size content at new positions.
        GdiFlush().ok()?;
        UpdateLayeredWindow(
            hwnd,
            None,
            Some(&POINT {
                x: rect.left,
                y: rect.top,
            }),
            Some(&self.size),
            Some(self.dc),
            Some(&POINT::default()),
            COLORREF(0),
            None,
            ULW_COLORKEY,
        )
    }
}
impl Drop for Surface {
    fn drop(&mut self) {
        unsafe {
            let _ = GdiFlush();
            if !self.old.is_invalid() {
                SelectObject(self.dc, self.old);
            }
            if !self.bitmap.is_invalid() {
                let _ = DeleteObject(self.bitmap.into());
            }
            let _ = DeleteDC(self.dc);
        }
    }
}

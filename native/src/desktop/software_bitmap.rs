//! Top-down BGRA DIB for software presentation; restore the DC's previous selected object before releasing bitmap/DC/pixel budgets.
use std::{marker::PhantomData, ptr::NonNull, rc::Rc};
use windows::{
    core::{Error, Result},
    Win32::{
        Foundation::*,
        Graphics::{Direct2D::Common::D2D_SIZE_U, Gdi::*},
    },
};

pub(super) struct Bitmap {
    old: HGDIOBJ,
    _bitmap: OwnedBitmap,
    dc: MemoryDc,
    _bits: NonNull<u8>,
    pub size: D2D_SIZE_U,
    _lease: crate::byte_budget::Lease,
    _thread: PhantomData<Rc<()>>,
}
#[path = "software_bitmap_frame.rs"]
mod frame;
impl Bitmap {
    pub fn new(size: D2D_SIZE_U) -> Result<Self> {
        let bytes = byte_count(size)?;
        Self::allocate(size, bytes)
    }
    fn allocate(size: D2D_SIZE_U, bytes: usize) -> Result<Self> {
        let lease = crate::pixels::reserve(bytes)
            .ok_or_else(|| Error::new(E_OUTOFMEMORY, "Software frame pixel budget exhausted"))?;
        unsafe {
            let dc = MemoryDc(CreateCompatibleDC(None));
            if dc.0.is_invalid() {
                return Err(Error::new(E_FAIL, "Software memory DC creation failed"));
            }
            let info = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: size.width as i32,
                    biHeight: -(size.height as i32),
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: BI_RGB.0,
                    ..Default::default()
                },
                ..Default::default()
            };
            let mut bits = std::ptr::null_mut();
            let bitmap = OwnedBitmap(CreateDIBSection(
                None,
                &info,
                DIB_RGB_COLORS,
                &mut bits,
                None,
                0,
            )?);
            let bits = NonNull::new(bits.cast::<u8>())
                .ok_or_else(|| Error::new(E_FAIL, "Missing software DIB storage"))?;
            // Performance audit optimization (Tier 0.4):
            // `CreateDIBSection` with `hSection = None` allocates fresh demand-zeroed anonymous pages from the Windows
            // kernel; skipping a redundant user-mode `write_bytes(0, bytes)` saves 31.64 MiB of page-fault write
            // traffic per 4K software surface before D2D/caller rendering immediately overwrites or clears it.
            let old = SelectObject(dc.0, bitmap.0.into());
            if old.is_invalid() {
                return Err(Error::new(E_FAIL, "Software DIB selection failed"));
            }
            Ok(Self {
                old,
                _bitmap: bitmap,
                dc,
                _bits: bits,
                size,
                _lease: lease,
                _thread: PhantomData,
            })
        }
    }
    pub fn dc(&self) -> HDC {
        self.dc.0
    }
    /// Read only after EndDraw succeeds; drain this thread's GDI batch without exposing DIB pointers or changing ownership.
    pub fn copy_into(&self, output: &mut [u8]) -> Result<()> {
        if output.len() != byte_count(self.size)? {
            return Err(Error::new(E_INVALIDARG, "Software DIB copy size mismatch"));
        }
        unsafe {
            GdiFlush().ok()?;
            std::ptr::copy_nonoverlapping(self._bits.as_ptr(), output.as_mut_ptr(), output.len());
        }
        Ok(())
    }
    #[cfg(test)]
    pub fn pixels(&self) -> Vec<[u8; 4]> {
        unsafe {
            std::slice::from_raw_parts(
                self._bits.as_ptr().cast(),
                (self.size.width * self.size.height) as usize,
            )
            .to_vec()
        }
    }
}
impl Drop for Bitmap {
    fn drop(&mut self) {
        unsafe {
            // A panic path may have completed only EndDraw; drain this thread's GDI batch before releasing the DIB.
            let _ = GdiFlush();
            let _ = SelectObject(self.dc.0, self.old);
        }
    }
}
struct OwnedBitmap(HBITMAP);
impl Drop for OwnedBitmap {
    fn drop(&mut self) {
        unsafe {
            let _ = DeleteObject(self.0.into());
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
pub(super) fn byte_count(size: D2D_SIZE_U) -> Result<usize> {
    let count = u64::from(size.width) * u64::from(size.height);
    if size.width == 0
        || size.height == 0
        || size.width > 8192
        || size.height > 8192
        || count > 16_777_216
    {
        return Err(Error::new(
            E_INVALIDARG,
            "Software surface exceeds presentation budget",
        ));
    }
    Ok(count as usize * 4)
}

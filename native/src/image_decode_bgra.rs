//! straight BGRA到PBGRA的唯一受预算转换；Relay与Wall共用，不保留透明像素隐藏RGB。
use super::*;
impl Image {
    pub(crate) fn from_bgra(bytes: &[u8], width: u32, height: u32) -> Result<Self> {
        if width == 0
            || height == 0
            || u64::from(width) * u64::from(height) * 4 != bytes.len() as u64
        {
            return Err(Error::new(E_INVALIDARG, "Invalid BGRA dimensions"));
        }
        let mut lease = crate::pixels::reserve(bytes.len()).ok_or_else(|| {
            Error::new(
                windows::Win32::Foundation::E_OUTOFMEMORY,
                "BGRA pixel budget exhausted",
            )
        })?;
        let mut pixels = Vec::new();
        pixels.try_reserve_exact(bytes.len()).map_err(|_| {
            Error::new(
                windows::Win32::Foundation::E_OUTOFMEMORY,
                "BGRA pixel allocation failed",
            )
        })?;
        if !lease.grow_to(pixels.capacity()) {
            return Err(Error::new(
                windows::Win32::Foundation::E_OUTOFMEMORY,
                "BGRA pixel capacity exceeds budget",
            ));
        }
        for p in bytes.chunks_exact(4) {
            let a = u16::from(p[3]);
            pixels.extend_from_slice(&[
                (u16::from(p[0]) * a / 255) as u8,
                (u16::from(p[1]) * a / 255) as u8,
                (u16::from(p[2]) * a / 255) as u8,
                p[3],
            ]);
        }
        Ok(Self {
            width,
            height,
            pixels: crate::pixels::Pixels::from_reserved(pixels, lease),
        })
    }
}

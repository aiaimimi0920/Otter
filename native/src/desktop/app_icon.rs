//! 内嵌品牌 ICO；目录有界验证，像素交给 Windows 解码，句柄晚于所有使用窗口释放。
use windows::{
    core::{Error, Result},
    Win32::{Foundation::*, UI::WindowsAndMessaging::*},
};
const SOURCE: &[u8] = include_bytes!("../../../src-tauri/icons/icon.ico");
const LIMIT: usize = 4 * 1024 * 1024;

fn frame(bytes: &[u8], desired: u32) -> Option<&[u8]> {
    if bytes.len() < 6
        || bytes.len() > LIMIT
        || bytes[..4] != [0, 0, 1, 0]
        || !(1..=256).contains(&desired)
    {
        return None;
    }
    let count = u16::from_le_bytes([bytes[4], bytes[5]]) as usize;
    if !(1..=256).contains(&count) {
        return None;
    }
    let end = 6 + count * 16;
    let directory = bytes.get(6..end)?;
    let mut best = None;
    for entry in directory.chunks_exact(16) {
        let width = if entry[0] == 0 { 256 } else { entry[0] as u32 };
        let height = if entry[1] == 0 { 256 } else { entry[1] as u32 };
        let length = u32::from_le_bytes(entry[8..12].try_into().ok()?) as usize;
        let offset = u32::from_le_bytes(entry[12..16].try_into().ok()?) as usize;
        if entry[3] != 0 || offset < end || length == 0 {
            return None;
        }
        let data = bytes.get(offset..offset.checked_add(length)?)?;
        // 优先选择足够大的帧再缩小；不将高分辨率资产上采样为模糊图标。
        let score = (
            width < desired || height < desired,
            width.abs_diff(desired) + height.abs_diff(desired),
        );
        if best.as_ref().is_none_or(|(old, _)| score < *old) {
            best = Some((score, data));
        }
    }
    best.map(|(_, data)| data)
}

struct Owned(HICON);
impl Drop for Owned {
    fn drop(&mut self) {
        unsafe {
            let _ = DestroyIcon(self.0);
        }
    }
}
impl Owned {
    fn new(size: u32) -> Result<Self> {
        let bytes =
            frame(SOURCE, size).ok_or_else(|| Error::new(E_INVALIDARG, "Invalid embedded icon"))?;
        // CreateIconFromResourceEx 要求 DWORD 对齐；ICO 文件偏移不保证满足要求。
        let mut aligned = vec![0u32; bytes.len().div_ceil(4)];
        let data = unsafe {
            std::slice::from_raw_parts_mut(aligned.as_mut_ptr().cast::<u8>(), bytes.len())
        };
        data.copy_from_slice(bytes);
        unsafe {
            CreateIconFromResourceEx(
                data,
                true,
                0x00030000,
                size as i32,
                size as i32,
                LR_DEFAULTCOLOR,
            )
            .map(Self)
        }
    }
}
pub(super) struct Icons {
    small: Owned,
    big: Owned,
}
impl Icons {
    pub(super) fn new() -> Result<Self> {
        Ok(Self {
            small: Owned::new(16)?,
            big: Owned::new(32)?,
        })
    }
    pub(super) unsafe fn attach(&self, hwnd: HWND) {
        SendMessageW(
            hwnd,
            WM_SETICON,
            Some(WPARAM(ICON_SMALL as usize)),
            Some(LPARAM(self.small.0 .0 as isize)),
        );
        SendMessageW(
            hwnd,
            WM_SETICON,
            Some(WPARAM(ICON_BIG as usize)),
            Some(LPARAM(self.big.0 .0 as isize)),
        );
    }
    pub(super) unsafe fn verify(&self, hwnd: HWND) -> Result<()> {
        for (kind, icon) in [(ICON_SMALL, self.small.0), (ICON_BIG, self.big.0)] {
            if SendMessageW(hwnd, WM_GETICON, Some(WPARAM(kind as usize)), None).0
                != icon.0 as isize
            {
                return Err(Error::new(E_FAIL, "Window icon binding mismatch"));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn embedded_directory_is_bounded_and_rejects_truncation_and_bad_offsets() {
        assert!(frame(SOURCE, 16).is_some());
        assert!(frame(SOURCE, 256).is_some());
        for length in 0..SOURCE.len() {
            assert!(frame(&SOURCE[..length], 32).is_none());
        }
        for offset in [0, 6, u32::MAX] {
            let mut bytes = SOURCE.to_vec();
            bytes[18..22].copy_from_slice(&offset.to_le_bytes());
            assert!(frame(&bytes, 32).is_none());
        }
        assert!(frame(SOURCE, 0).is_none());
        assert!(frame(SOURCE, 257).is_none());
    }
    #[test]
    fn native_decoder_creates_requested_sizes_and_owned_handles() {
        use windows::Win32::Graphics::Gdi::*;
        unsafe {
            for size in [16, 24, 32, 48, 64, 256] {
                let icon = Owned::new(size).unwrap();
                let mut info = ICONINFO::default();
                GetIconInfo(icon.0, &mut info).unwrap();
                let mut bitmap = BITMAP::default();
                let result = GetObjectW(
                    info.hbmColor.into(),
                    std::mem::size_of::<BITMAP>() as i32,
                    Some((&mut bitmap as *mut BITMAP).cast()),
                );
                let _ = DeleteObject(info.hbmColor.into());
                let _ = DeleteObject(info.hbmMask.into());
                assert_ne!(result, 0);
                assert_eq!(
                    (bitmap.bmWidth, bitmap.bmHeight),
                    (size as i32, size as i32)
                );
            }
        }
    }
}

//! 只响应明确复制命令；分配/编码先完成，SetClipboardData 成功后所有权交给系统。
use super::image::Image;
#[path = "clipboard_text.rs"]
mod text;
pub(super) use text::copy_checked as copy_text_checked;
use windows::{
    core::{Error, Result},
    Win32::{
        Foundation::*,
        System::{DataExchange::*, Memory::*},
    },
};

struct Clipboard;
impl Drop for Clipboard {
    fn drop(&mut self) {
        unsafe {
            let _ = CloseClipboard();
        }
    }
}
struct Memory(HGLOBAL);
impl Drop for Memory {
    fn drop(&mut self) {
        unsafe {
            let _ = GlobalFree(Some(self.0));
        }
    }
}
struct Locked(HGLOBAL);
impl Drop for Locked {
    fn drop(&mut self) {
        unsafe {
            let _ = GlobalUnlock(self.0);
        }
    }
}

struct Prepared {
    // 按字段顺序先释放尚未交付的 HGLOBAL，再归还预算。
    memory: Option<Memory>,
    _lease: crate::byte_budget::Lease,
}
impl Prepared {
    unsafe fn transfer(mut self, format: u32) -> Result<()> {
        SetClipboardData(format, Some(HANDLE(self.memory.as_ref().unwrap().0 .0)))?;
        std::mem::forget(self.memory.take().unwrap());
        Ok(())
    }
}

fn dib_size(image: &Image, rect: RECT) -> Result<usize> {
    if rect.left < 0
        || rect.top < 0
        || rect.right <= rect.left
        || rect.bottom <= rect.top
        || rect.right > image.width as i32
        || rect.bottom > image.height as i32
        || crate::viewport::checked_image_bytes(image.width, image.height)
            != Some(image.pixels.len())
    {
        return Err(Error::new(
            E_INVALIDARG,
            "Invalid clipboard image rectangle",
        ));
    }
    let count = crate::viewport::checked_image_bytes(
        (rect.right - rect.left) as u32,
        (rect.bottom - rect.top) as u32,
    )
    .ok_or_else(|| Error::new(E_INVALIDARG, "Clipboard image exceeds budget"))?;
    Ok(count + 40)
}
fn encode_dib(image: &Image, rect: RECT, bytes: &mut [u8]) -> Result<()> {
    let count = dib_size(image, rect)?;
    if bytes.len() != count {
        return Err(Error::new(E_INVALIDARG, "Invalid clipboard buffer size"));
    }
    let width = rect.right - rect.left;
    let height = rect.bottom - rect.top;
    bytes[..40].fill(0);
    bytes[..4].copy_from_slice(&40u32.to_le_bytes());
    bytes[4..8].copy_from_slice(&width.to_le_bytes());
    // 顶向下 BI_RGB DIB，不通过反转整图产生另一份缓冲。
    bytes[8..12].copy_from_slice(&(-height).to_le_bytes());
    bytes[12..14].copy_from_slice(&1u16.to_le_bytes());
    bytes[14..16].copy_from_slice(&32u16.to_le_bytes());
    bytes[20..24].copy_from_slice(&((count - 40) as u32).to_le_bytes());
    let stride = image.width as usize * 4;
    for (row, destination) in bytes[40..].chunks_exact_mut(width as usize * 4).enumerate() {
        let start = (rect.top as usize + row) * stride + rect.left as usize * 4;
        destination.copy_from_slice(&image.pixels[start..start + destination.len()]);
        for pixel in destination.chunks_exact_mut(4) {
            pixel[3] = 255;
        }
    }
    Ok(())
}
unsafe fn prepare(size: usize, fill: impl FnOnce(&mut [u8]) -> Result<()>) -> Result<Prepared> {
    let mut lease = crate::pixels::reserve(size)
        .ok_or_else(|| Error::new(E_OUTOFMEMORY, "Clipboard staging budget exhausted"))?;
    let memory = Memory(GlobalAlloc(GMEM_MOVEABLE, size)?);
    if !lease.grow_to(GlobalSize(memory.0)) {
        return Err(Error::new(
            E_OUTOFMEMORY,
            "Clipboard allocation exceeds budget",
        ));
    }
    let pointer = GlobalLock(memory.0);
    if pointer.is_null() {
        return Err(Error::from_win32());
    }
    let lock = Locked(memory.0);
    fill(std::slice::from_raw_parts_mut(pointer.cast::<u8>(), size))?;
    drop(lock);
    Ok(Prepared {
        memory: Some(memory),
        _lease: lease,
    })
}
unsafe fn publish(
    owner: HWND,
    format: u32,
    size: usize,
    fill: impl FnOnce(&mut [u8]) -> Result<()>,
) -> Result<()> {
    let prepared = prepare(size, fill)?;
    // 失败前不触碰系统剪贴板。EmptyClipboard 后的发布失败无法恢复第三方格式。
    OpenClipboard(Some(owner))?;
    let _clipboard = Clipboard;
    EmptyClipboard()?;
    prepared.transfer(format)
}
pub(super) unsafe fn copy_image(owner: HWND, image: &Image, rect: RECT) -> Result<()> {
    let size = dib_size(image, rect)?;
    publish(owner, 8, size, |bytes| encode_dib(image, rect, bytes)) // CF_DIB
}
/// 文件须由调用方先写完并保持有效；本层只转移剪贴板内存，不负责磁盘文件寿命。
/// EmptyClipboard 后可能部分发布，失败也不能据此立即删除已准备的文件。
pub(super) unsafe fn copy_formats_checked(
    owner: HWND,
    payload: &super::clipboard_bitmap::Payload,
    files: &[std::path::PathBuf],
    expected_sequence: Option<u32>,
) -> Result<Option<u32>> {
    let prepared = prepare_formats(payload, files)?;
    let format = RegisterClipboardFormatW(windows::core::w!("PNG"));
    if format == 0 {
        return Err(Error::from_win32());
    }
    OpenClipboard(Some(owner))?;
    let _clipboard = Clipboard;
    // 持锁后再核对，覆盖缓存发布及 HGLOBAL 准备期间发生的外部更新。
    publish_checked(
        expected_sequence,
        || GetClipboardSequenceNumber(),
        || {
            EmptyClipboard()?;
            prepared.png.transfer(format)?;
            prepared.dib.transfer(17)?; // CF_DIBV5
            if let Some(files) = prepared.files {
                files.transfer(15)?;
            } // CF_HDROP
            Ok(())
        },
    )
}

fn publish_checked(
    expected: Option<u32>,
    sequence: impl Fn() -> u32,
    transfer: impl FnOnce() -> Result<()>,
) -> Result<Option<u32>> {
    if !sequence_allows(expected, &sequence) {
        return Ok(None);
    }
    transfer()?;
    // Immediate rendering only: capture the receipt before releasing the lock.
    Ok(Some(sequence()))
}

struct PreparedFormats {
    png: Prepared,
    dib: Prepared,
    files: Option<Prepared>,
}
fn sequence_allows(expected: Option<u32>, current: impl FnOnce() -> u32) -> bool {
    expected.is_none_or(|expected| current() == expected)
}
unsafe fn prepare_formats(
    payload: &super::clipboard_bitmap::Payload,
    files: &[std::path::PathBuf],
) -> Result<PreparedFormats> {
    let png = &payload.png;
    if png.len() > 257 * 1024 * 1024 || !png.starts_with(b"\x89PNG\r\n\x1a\n") {
        return Err(Error::new(E_INVALIDARG, "Invalid clipboard PNG"));
    }
    let file_bytes = if files.is_empty() {
        None
    } else {
        Some(super::paste_input::file::encode(files)?)
    };
    // 所有 HGLOBAL 都先准备好；一次 Open/Empty 发布所有格式，不逐个清空。
    let png_memory = prepare(png.len(), |bytes| {
        bytes.copy_from_slice(png);
        Ok(())
    })?;
    let dib_memory = prepare(payload.dibv5.len(), |bytes| {
        bytes.copy_from_slice(&payload.dibv5);
        Ok(())
    })?;
    let files = file_bytes
        .as_ref()
        .map(|data| {
            prepare(data.len(), |bytes| {
                bytes.copy_from_slice(data);
                Ok(())
            })
        })
        .transpose()?;
    Ok(PreparedFormats {
        png: png_memory,
        dib: dib_memory,
        files,
    })
}
pub(super) unsafe fn copy_color(owner: HWND, rgb: [u8; 3]) -> Result<()> {
    let text = format!("#{:02X}{:02X}{:02X}", rgb[0], rgb[1], rgb[2]);
    let units: Vec<u16> = text.encode_utf16().chain(Some(0)).collect();
    publish(owner, 13, units.len() * 2, |bytes| {
        // CF_UNICODETEXT
        for (output, unit) in bytes.chunks_exact_mut(2).zip(units) {
            output.copy_from_slice(&unit.to_le_bytes());
        }
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn publication_receipt_requires_all_transfers_and_uses_post_publish_sequence() {
        let sequence = std::cell::Cell::new(41);
        assert_eq!(
            publish_checked(Some(40), || sequence.get(), || panic!("stale publication")).unwrap(),
            None
        );
        assert_eq!(
            publish_checked(
                Some(41),
                || sequence.get(),
                || {
                    sequence.set(44);
                    Ok(())
                }
            )
            .unwrap(),
            Some(44)
        );
        assert!(publish_checked(
            Some(44),
            || sequence.get(),
            || {
                sequence.set(45);
                Err(Error::new(E_FAIL, "partial transfer"))
            }
        )
        .is_err());
    }
    #[test]
    fn final_sequence_guard_rejects_changes_without_reading_clipboard_contents() {
        assert!(sequence_allows(None, || panic!(
            "unguarded synchronous path does not query"
        )));
        assert!(sequence_allows(Some(17), || 17));
        assert!(!sequence_allows(Some(17), || 18));
        assert!(sequence_allows(Some(0), || 0));
    }
    #[test]
    fn file_list_is_prepared_with_image_formats_without_opening_clipboard() {
        let payload = super::super::clipboard_bitmap::Payload {
            png: b"\x89PNG\r\n\x1a\n".to_vec().into(),
            dibv5: vec![0; 124].into(),
        };
        let paths = vec![std::path::PathBuf::from("C:/不存在/图片🙂.png")];
        let expected = super::super::paste_input::file::encode(&paths).unwrap();
        unsafe {
            let all = prepare_formats(&payload, &paths).unwrap();
            let memory = all.files.as_ref().unwrap().memory.as_ref().unwrap().0;
            let pointer = GlobalLock(memory);
            assert!(!pointer.is_null());
            let lock = Locked(memory);
            assert_eq!(
                std::slice::from_raw_parts(pointer.cast::<u8>(), expected.len()),
                expected
            );
            drop(lock);
            drop(all); // 所有尚未转移的 HGLOBAL 由 Prepared 回收。
            assert!(prepare_formats(&payload, &[]).unwrap().files.is_none());
            assert!(
                prepare_formats(&payload, &[std::path::PathBuf::from("relative.png")]).is_err()
            );
        }
    }
    #[test]
    fn cropped_dib_has_exact_header_and_top_down_opaque_bgra_without_clipboard_access() {
        let image = Image {
            width: 3,
            height: 2,
            pixels: (0..24).collect::<Vec<u8>>().into(),
        };
        let rect = RECT {
            left: 1,
            top: 0,
            right: 3,
            bottom: 2,
        };
        let mut bytes = vec![0; dib_size(&image, rect).unwrap()];
        encode_dib(&image, rect, &mut bytes).unwrap();
        assert_eq!(bytes.len(), 56);
        assert_eq!(&bytes[..4], &40u32.to_le_bytes());
        assert_eq!(&bytes[4..8], &2i32.to_le_bytes());
        assert_eq!(&bytes[8..12], &(-2i32).to_le_bytes());
        assert_eq!(&bytes[12..16], &[1, 0, 32, 0]);
        assert_eq!(
            &bytes[40..],
            &[4, 5, 6, 255, 8, 9, 10, 255, 16, 17, 18, 255, 20, 21, 22, 255]
        );
        assert!(encode_dib(&image, rect, &mut bytes[..55]).is_err());
        assert!(dib_size(&image, RECT { left: -1, ..rect }).is_err());
    }
}

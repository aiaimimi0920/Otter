//! CF_HDROP 的有界文件列表解析；ANSI 使用系统代码页，不调用 DragFinish、不访问文件系统。
use std::{ffi::OsString, os::windows::ffi::OsStringExt, path::PathBuf};
use windows::core::{Error, Result};
use windows::Win32::Foundation::E_INVALIDARG;
use windows::Win32::Globalization::{MultiByteToWideChar, CP_ACP, MB_ERR_INVALID_CHARS};

pub(super) const MAX_BYTES: usize = 128 * 1024;
#[path = "clipboard_file_encode.rs"]
mod output;
pub(in crate::desktop) use output::encode;
pub(super) fn parse(bytes: &[u8]) -> Result<Vec<PathBuf>> {
    let invalid = || {
        Error::new(
            E_INVALIDARG,
            "剪贴板文件列表无效或超出 32 个文件的批次限制。",
        )
    };
    if bytes.len() < 22 || bytes.len() > MAX_BYTES {
        return Err(invalid());
    }
    let offset = u32::from_le_bytes(bytes[..4].try_into().unwrap()) as usize;
    let wide = u32::from_le_bytes(bytes[16..20].try_into().unwrap());
    if offset < 20 || (wide != 0 && !offset.is_multiple_of(2)) {
        return Err(invalid());
    }
    let data = bytes.get(offset..).ok_or_else(invalid)?;
    let mut paths = Vec::new();
    if wide == 0 {
        let mut remaining = data;
        loop {
            let end = remaining
                .iter()
                .position(|byte| *byte == 0)
                .ok_or_else(invalid)?;
            if end == 0 {
                return if paths.is_empty() {
                    Err(invalid())
                } else {
                    Ok(paths)
                };
            }
            if paths.len() == super::super::open_job::MAX_BATCH {
                return Err(invalid());
            }
            paths.push(ansi_path(&remaining[..end], CP_ACP)?);
            remaining = &remaining[end + 1..];
        }
    }
    let mut units = data
        .chunks_exact(2)
        .map(|pair| u16::from_le_bytes([pair[0], pair[1]]));
    loop {
        let mut path = Vec::new();
        loop {
            let unit = units.next().ok_or_else(invalid)?;
            if unit == 0 {
                break;
            }
            if path.len() == 32767 {
                return Err(invalid());
            }
            path.push(unit);
        }
        if path.is_empty() {
            return if paths.is_empty() {
                Err(invalid())
            } else {
                Ok(paths)
            };
        }
        if paths.len() == super::super::open_job::MAX_BATCH {
            return Err(invalid());
        }
        paths.push(PathBuf::from(OsString::from_wide(&path)));
    }
}

fn ansi_path(bytes: &[u8], code_page: u32) -> Result<PathBuf> {
    // 先查询 UTF-16 长度，拒绝损坏的多字节序列，避免替换字符意外指向另一文件。
    let length = unsafe { MultiByteToWideChar(code_page, MB_ERR_INVALID_CHARS, bytes, None) };
    if length == 0 {
        return Err(Error::from_win32());
    }
    if length > 32767 {
        return Err(Error::new(
            E_INVALIDARG,
            "Clipboard path exceeds length limit",
        ));
    }
    let mut path = vec![0; length as usize];
    let written =
        unsafe { MultiByteToWideChar(code_page, MB_ERR_INVALID_CHARS, bytes, Some(&mut path)) };
    if written != length {
        return Err(Error::from_win32());
    }
    Ok(PathBuf::from(OsString::from_wide(&path)))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn packet(names: &[&str]) -> Vec<u8> {
        let mut bytes = vec![0; 20];
        bytes[..4].copy_from_slice(&20u32.to_le_bytes());
        bytes[16..20].copy_from_slice(&1u32.to_le_bytes());
        for name in names {
            for unit in name.encode_utf16().chain([0]) {
                bytes.extend_from_slice(&unit.to_le_bytes());
            }
        }
        bytes.extend_from_slice(&[0, 0]);
        bytes
    }
    #[test]
    fn unicode_path_is_copied_without_file_access() {
        let name = "C:\\不存在的目录\\中文 🦦 image.png";
        let bytes = packet(&[name]);
        assert_eq!(parse(&bytes).unwrap(), vec![PathBuf::from(name)]);
        for length in 0..bytes.len() {
            assert!(parse(&bytes[..length]).is_err());
        }
        let mut padded = bytes.clone();
        padded.extend_from_slice(&[42; 17]);
        assert_eq!(parse(&padded).unwrap(), vec![PathBuf::from(name)]);
    }
    #[test]
    fn parsed_repository_fixture_opens_on_worker_without_clipboard_access() {
        let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("src-tauri/icons/32x32.png");
        let path = parse(&packet(&[fixture.to_str().unwrap()]))
            .unwrap()
            .pop()
            .unwrap();
        let prepared = std::thread::spawn(move || {
            let _apartment = crate::desktop::Apartment::new().unwrap();
            crate::desktop::open_image::PreparedDocument::open(&path, false)
        })
        .join()
        .unwrap()
        .unwrap();
        assert_eq!((prepared.image.width, prepared.image.height), (32, 32));
    }
    #[test]
    fn batches_preserve_order_and_reject_invalid_headers_and_unbounded_paths() {
        assert!(parse(&packet(&[])).is_err());
        assert_eq!(
            parse(&packet(&["a", "b"])).unwrap(),
            vec![PathBuf::from("a"), PathBuf::from("b")]
        );
        assert!(parse(&packet(&["a"; 33])).is_err());
        assert!(parse(&packet(&[&"a".repeat(32768)])).is_err());
        assert!(parse(&vec![0; MAX_BYTES + 1]).is_err());
        for offset in [0u32, 19, 21, u32::MAX] {
            let mut bytes = packet(&["a"]);
            bytes[..4].copy_from_slice(&offset.to_le_bytes());
            assert!(parse(&bytes).is_err());
        }
    }
    #[test]
    fn ansi_packet_uses_system_code_page_and_requires_complete_list() {
        let mut packet = vec![0; 20];
        packet[..4].copy_from_slice(&20u32.to_le_bytes());
        packet.extend_from_slice(b"C:\\image.png\0\0");
        assert_eq!(
            parse(&packet).unwrap(),
            vec![PathBuf::from("C:\\image.png")]
        );
        for length in 0..packet.len() {
            assert!(parse(&packet[..length]).is_err());
        }
        packet.pop();
        packet.extend_from_slice(b"second.png\0\0");
        assert_eq!(
            parse(&packet).unwrap(),
            vec![PathBuf::from("C:\\image.png"), PathBuf::from("second.png")]
        );
    }
    #[test]
    fn ansi_conversion_is_strict_and_handles_dbcs_without_changing_system_locale() {
        assert_eq!(
            ansi_path(b"C:\\\xd6\xd0\xce\xc4.png", 936).unwrap(),
            PathBuf::from("C:\\中文.png")
        );
        assert_eq!(
            ansi_path(b"C:\\caf\xe9.png", 1252).unwrap(),
            PathBuf::from("C:\\café.png")
        );
        assert!(ansi_path(b"\xd6", 936).is_err());
        assert!(ansi_path(&vec![b'a'; 32768], 1252).is_err());
    }
    #[test]
    fn multi_file_terminator_and_count_are_validated_for_both_encodings() {
        let wide = packet(&["C:\\中文 🦦.png", "C:\\second.png"]);
        for length in 0..wide.len() {
            assert!(parse(&wide[..length]).is_err());
        }
        assert_eq!(parse(&packet(&["x.png"; 32])).unwrap().len(), 32);
        let mut ansi = vec![0; 20];
        ansi[..4].copy_from_slice(&20u32.to_le_bytes());
        ansi.extend_from_slice(b"C:\\a.png\0C:\\b.png\0\0");
        for length in 0..ansi.len() {
            assert!(parse(&ansi[..length]).is_err());
        }
        let mut oversized = vec![0; 20];
        oversized[..4].copy_from_slice(&20u32.to_le_bytes());
        oversized.extend_from_slice("x.png\0".repeat(33).as_bytes());
        oversized.push(0);
        assert!(parse(&oversized).is_err());
    }
}

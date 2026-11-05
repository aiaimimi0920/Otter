//! Explorer 文件选择缓冲解析；单文件完整路径，多文件为目录加有界文件名列表。
use std::{
    ffi::OsString,
    os::windows::ffi::{OsStrExt, OsStringExt},
    path::PathBuf,
};
use windows::{
    core::{Error, Result},
    Win32::Foundation::E_INVALIDARG,
};
const MAX_PATH_UNITS: usize = 32767;
pub(super) const MAX_BUFFER: usize = 32768 * (super::super::open_job::MAX_BATCH + 1);

fn invalid() -> Error {
    Error::new(E_INVALIDARG, "文件选择结果不完整、路径非法或超出批次限制。")
}
fn part(buffer: &[u16], offset: usize) -> Result<(&[u16], usize)> {
    let remaining = buffer.get(offset..).ok_or_else(invalid)?;
    let length = remaining
        .iter()
        .position(|unit| *unit == 0)
        .ok_or_else(invalid)?;
    if length > MAX_PATH_UNITS {
        return Err(invalid());
    }
    Ok((&remaining[..length], offset + length + 1))
}
pub(super) fn parse(buffer: &[u16], file_offset: usize, multiple: bool) -> Result<Vec<PathBuf>> {
    if buffer.len() > MAX_BUFFER {
        return Err(invalid());
    }
    let (first, mut next) = part(buffer, 0)?;
    if first.is_empty() {
        return Err(invalid());
    }
    let parent = PathBuf::from(OsString::from_wide(first));
    if !parent.is_absolute() {
        return Err(invalid());
    }
    if file_offset > 0 && file_offset < first.len() {
        let name = parent.file_name().ok_or_else(invalid)?;
        if file_offset != first.len() - name.encode_wide().count() {
            return Err(invalid());
        }
        return Ok(vec![parent]);
    }
    if !multiple || file_offset != next {
        return Err(invalid());
    }
    let mut paths = Vec::new();
    loop {
        let (name, end) = part(buffer, next)?;
        if name.is_empty() {
            break;
        }
        if paths.len() >= super::super::open_job::MAX_BATCH
            || name.iter().any(|unit| matches!(*unit, 47 | 92 | 58))
            || name == [46]
            || name == [46, 46]
        {
            return Err(invalid());
        }
        let path = parent.join(OsString::from_wide(name));
        if path.as_os_str().encode_wide().count() > MAX_PATH_UNITS {
            return Err(invalid());
        }
        paths.push(path);
        next = end;
    }
    if paths.is_empty() {
        Err(invalid())
    } else {
        Ok(paths)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn wide(value: &str) -> Vec<u16> {
        value.encode_utf16().collect()
    }
    #[test]
    fn single_and_multi_unicode_paths_keep_order_and_spaces() {
        assert_eq!(
            parse(&wide("C:\\图 片\\水獭.png\0"), 7, true).unwrap(),
            vec![PathBuf::from("C:\\图 片\\水獭.png")]
        );
        let directory = "C:\\图 片";
        let buffer = wide(&format!("{directory}\0水獭.png\0第二 张.jpg\0\0"));
        assert_eq!(
            parse(&buffer, directory.encode_utf16().count() + 1, true).unwrap(),
            vec![
                PathBuf::from("C:\\图 片\\水獭.png"),
                PathBuf::from("C:\\图 片\\第二 张.jpg")
            ]
        );
    }
    #[test]
    fn truncated_empty_bad_offset_and_replacement_batches_are_rejected() {
        for (buffer, offset, multi) in [
            ("", 0, true),
            ("C:\\a", 3, true),
            ("C:\\a\0x.png\0", 5, true),
            ("C:\\a\0\0", 5, true),
            ("C:\\a\0x.png\0\0", 6, true),
            ("C:\\a\0x.png\0\0", 5, false),
            ("relative.png\0", 1, true),
        ] {
            assert!(parse(&wide(buffer), offset, multi).is_err(), "{buffer:?}");
        }
    }
    #[test]
    fn child_paths_cannot_escape_directory_or_exceed_count_and_length() {
        for name in ["..", ".", "..\\x.png", "C:x.png", "\\x.png", "a/b.png"] {
            assert!(parse(&wide(&format!("C:\\a\0{name}\0\0")), 5, true).is_err());
        }
        let many = format!("C:\\a\0{}\0", "x.png\0".repeat(33));
        assert!(parse(&wide(&many), 5, true).is_err());
        let long = format!("C:\\a\0{}\0\0", "x".repeat(MAX_PATH_UNITS));
        assert!(parse(&wide(&long), 5, true).is_err());
    }
}

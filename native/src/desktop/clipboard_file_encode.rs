//! CF_HDROP 输出合同；只编码调用方拥有的绝对路径，不读取文件、不隐式授予目录权限。
use std::{
    os::windows::ffi::OsStrExt,
    path::{Component, PathBuf, Prefix},
};
use windows::{
    core::{Error, Result},
    Win32::Foundation::E_INVALIDARG,
};
pub(in crate::desktop) fn encode(paths: &[PathBuf]) -> Result<Vec<u8>> {
    let invalid = || Error::new(E_INVALIDARG, "Invalid clipboard output file list");
    if paths.is_empty() || paths.len() > crate::desktop::open_job::MAX_BATCH {
        return Err(invalid());
    }
    let mut size = 22usize; // DROPFILES + 最后一个空字符串；每条路径另含 NUL。
    for path in paths {
        if !path.is_absolute()
            || !matches!(path.components().next(), Some(Component::Prefix(prefix)) if matches!(prefix.kind(), Prefix::Disk(_) | Prefix::UNC(_, _)))
            || path
                .components()
                .any(|part| matches!(part, Component::ParentDir))
        {
            return Err(invalid());
        }
        let mut count = 0;
        for unit in path.as_os_str().encode_wide() {
            if unit == 0 || count == 32767 {
                return Err(invalid());
            }
            count += 1;
        }
        size = size.checked_add((count + 1) * 2).ok_or_else(invalid)?;
        if size > super::MAX_BYTES {
            return Err(invalid());
        }
    }
    let mut bytes = vec![0; 20];
    bytes.reserve_exact(size - 20);
    bytes[..4].copy_from_slice(&20u32.to_le_bytes());
    bytes[16..20].copy_from_slice(&1u32.to_le_bytes());
    for path in paths {
        for unit in path.as_os_str().encode_wide().chain([0]) {
            bytes.extend_from_slice(&unit.to_le_bytes());
        }
    }
    bytes.extend_from_slice(&[0, 0]);
    debug_assert_eq!(bytes.len(), size);
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn wide_packet_round_trips_unicode_unc_and_multiple_paths_without_disk_access() {
        let paths = vec![
            PathBuf::from("C:/不存在/中文 🦦.png"),
            PathBuf::from(r"\\host\share\image.png"),
        ];
        let bytes = encode(&paths).unwrap();
        assert_eq!(&bytes[..4], &20u32.to_le_bytes());
        assert_eq!(&bytes[4..16], &[0; 12]);
        assert_eq!(&bytes[16..20], &1u32.to_le_bytes());
        assert_eq!(&bytes[bytes.len() - 4..], &[0; 4]);
        assert_eq!(super::super::parse(&bytes).unwrap(), paths);
    }
    #[test]
    fn invalid_authority_nul_and_capacity_are_rejected_before_allocation() {
        for path in [
            "relative.png",
            "C:relative.png",
            "C:/x/../y.png",
            "C:/nul\0.png",
            r"\\?\C:\file.png",
            r"\\.\pipe\name",
        ] {
            assert!(encode(&[PathBuf::from(path)]).is_err(), "{path:?}");
        }
        assert!(encode(&[]).is_err());
        assert!(encode(&vec![PathBuf::from("C:/a.png"); 33]).is_err());
        assert!(encode(&[PathBuf::from(format!("C:/{}", "a".repeat(32768)))]).is_err());
        let long = PathBuf::from(format!("C:/{}", "中".repeat(32000)));
        assert!(encode(&[long.clone(), long.clone(), long]).is_err());
        assert!(encode(&vec![PathBuf::from("C:/a.png"); 32]).is_ok());
    }
}

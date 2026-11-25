//! 真 Loom 联合测试输入：从同一句柄核对路径、文件类型和读取上限，再交由各夹具解析。
use std::{
    ffi::OsString,
    fs::{File, OpenOptions},
    io::{self, Read},
    os::windows::{
        ffi::{OsStrExt, OsStringExt},
        fs::{MetadataExt, OpenOptionsExt},
        io::AsRawHandle,
    },
    path::{Component, Path, PathBuf, Prefix},
};
use windows::Win32::{Foundation::HANDLE, Storage::FileSystem::*};
use zeroize::Zeroizing;

const LIMIT: usize = 64 * 1024;

pub(crate) fn load(input: &Path) -> io::Result<(&Path, Zeroizing<Vec<u8>>)> {
    load_under(input, &std::env::temp_dir())
}

fn load_under<'a>(input: &'a Path, temp: &Path) -> io::Result<(&'a Path, Zeroizing<Vec<u8>>)> {
    if !input.is_absolute()
        || !matches!(input.components().next(), Some(Component::Prefix(p))
            if matches!(p.kind(), Prefix::Disk(b'C' | b'c')))
        || !input
            .file_name()
            .is_some_and(|name| !name.encode_wide().any(|unit| unit == u16::from(b':')))
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "invalid joint input path",
        ));
    }
    let root = input.parent().unwrap();
    let resolved_root = root.canonicalize()?;
    if !matches!(resolved_root.components().next(), Some(Component::Prefix(p))
        if matches!(p.kind(), Prefix::VerbatimDisk(b'C' | b'c') | Prefix::Disk(b'C' | b'c')))
        || !resolved_root.starts_with(temp.canonicalize()?)
    {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "joint input outside fixture root",
        ));
    }
    let file = OpenOptions::new()
        .read(true)
        .share_mode(FILE_SHARE_READ.0)
        .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT.0 | FILE_FLAG_SEQUENTIAL_SCAN.0)
        .open(input)?;
    let bytes = read_open(file, &resolved_root)?;
    // 保留启动脚本传入的普通路径；下游 profile 不接收 Win32 verbatim 路径。
    Ok((root, bytes))
}

fn read_open(file: File, expected_parent: &Path) -> io::Result<Zeroizing<Vec<u8>>> {
    let handle = HANDLE(file.as_raw_handle());
    if unsafe { GetFileType(handle) } != FILE_TYPE_DISK {
        return Err(invalid("joint input must be a disk file"));
    }
    let metadata = file.metadata()?;
    if !metadata.is_file() || metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT.0 != 0 {
        return Err(invalid("joint input must be a regular non-reparse file"));
    }
    if final_path(handle)?.parent() != Some(expected_parent) {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "joint input handle left fixture directory",
        ));
    }
    let length =
        usize::try_from(metadata.len()).map_err(|_| invalid("joint input exceeds limit"))?;
    read_exact_bounded(file, length)
}

fn final_path(handle: HANDLE) -> io::Result<PathBuf> {
    let query = |buf: &mut [u16]| unsafe {
        GetFinalPathNameByHandleW(
            handle,
            buf,
            GETFINALPATHNAMEBYHANDLE_FLAGS(FILE_NAME_NORMALIZED.0 | VOLUME_NAME_DOS.0),
        ) as usize
    };
    let mut short = [0; 260];
    let length = query(&mut short);
    match length {
        0 => Err(io::Error::last_os_error()),
        1..260 => Ok(OsString::from_wide(&short[..length]).into()),
        260..=32768 => {
            // 不足返回值包含 NUL；成功返回值不包含 NUL。
            let mut long = vec![0; length];
            match query(&mut long) {
                0 => Err(io::Error::last_os_error()),
                count if count < long.len() => Ok(OsString::from_wide(&long[..count]).into()),
                _ => Err(invalid("joint input final path exceeds limit")),
            }
        }
        _ => Err(invalid("joint input final path exceeds limit")),
    }
}

fn invalid(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

fn read_exact_bounded(mut source: impl Read, length: usize) -> io::Result<Zeroizing<Vec<u8>>> {
    if length > LIMIT {
        return Err(invalid("joint input exceeds limit"));
    }
    // Zeroize also covers a partially filled buffer on error; the extra byte is only an EOF probe.
    let mut bytes = Zeroizing::new(vec![0; length]);
    source.read_exact(&mut bytes)?;
    let mut probe = Zeroizing::new([0]);
    if source.read(&mut probe[..])? != 0 {
        return Err(invalid("joint input grew during read"));
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device_session_client::tests::pairing::Files;
    use std::fs;

    #[test]
    fn joint_input_real_handle_checks_directory_and_exact_size_limit() {
        let (allowed, _, _) = Files::new();
        let (other, _, _) = Files::new();
        let path = allowed.root().join("joint.json");
        fs::write(&path, vec![b' '; LIMIT]).unwrap();
        let (root, bytes) = load_under(&path, allowed.root()).unwrap();
        assert_eq!(root, allowed.root());
        assert_eq!(bytes.len(), LIMIT);
        assert!(bytes.iter().all(|byte| *byte == b' '));
        assert_eq!(
            read_open(
                File::open(&path).unwrap(),
                &other.root().canonicalize().unwrap()
            )
            .unwrap_err()
            .kind(),
            io::ErrorKind::PermissionDenied
        );
        assert_eq!(
            load_under(&path, other.root()).unwrap_err().kind(),
            io::ErrorKind::PermissionDenied
        );
        fs::write(&path, vec![b' '; LIMIT + 1]).unwrap();
        assert_eq!(
            load_under(&path, allowed.root()).unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );
    }

    #[test]
    fn joint_input_rejects_length_drift_without_returning_partial_bytes() {
        assert_eq!(
            read_exact_bounded(&b""[..], 1).unwrap_err().kind(),
            io::ErrorKind::UnexpectedEof
        );
        assert_eq!(
            read_exact_bounded(&b"{} "[..], 2).unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );
    }
}

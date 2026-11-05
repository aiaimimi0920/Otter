//! 原始 HDR PNG 按字节另存：读已有租约而非重开路径；显式偏移避免共享文件游标竞争。
use super::*;
pub(crate) fn encode_inline(
    source: &std::fs::File,
    cancel: &AtomicBool,
) -> Result<crate::loom_base64::Encoded, crate::loom_base64::Error> {
    struct Reader<'a> {
        file: &'a std::fs::File,
        offset: u64,
    }
    impl std::io::Read for Reader<'_> {
        fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
            let count = self.file.seek_read(bytes, self.offset)?;
            self.offset += count as u64;
            Ok(count)
        }
    }
    let length = source
        .metadata()
        .map_err(|_| crate::loom_base64::Error::Read)?
        .len();
    let length = usize::try_from(length).map_err(|_| crate::loom_base64::Error::Size)?;
    crate::loom_base64::encode_png_reader(
        Reader {
            file: source,
            offset: 0,
        },
        length,
        cancel,
    )
}
use std::{fs::File, io::Write, os::windows::fs::FileExt, sync::atomic::Ordering};
pub(crate) fn prepare(
    source: &File,
    destination: &Path,
    cancel: &AtomicBool,
) -> io::Result<Staged> {
    check(cancel)?;
    if !destination
        .extension()
        .is_some_and(|value| value.eq_ignore_ascii_case("png"))
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "HDR original destination must be PNG",
        ));
    }
    let metadata = source.metadata()?;
    let length = metadata.len();
    if !metadata.is_file() || !(8..=257 * 1024 * 1024).contains(&length) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Invalid HDR original file",
        ));
    }
    let mut signature = [0u8; 8];
    read_at(source, &mut signature, 0, cancel)?;
    if &signature != b"\x89PNG\r\n\x1a\n" {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Invalid HDR original signature",
        ));
    }
    // 所属捕获编码器已验证格式；此处保留字节，不尝试解码色彩或接受任意外部路径。
    Staged::prepare_stream(destination, cancel, |output, cancel| {
        let mut buffer = [0u8; 64 * 1024];
        let mut offset = 0;
        while offset < length {
            check(cancel)?;
            let count = (length - offset).min(buffer.len() as u64) as usize;
            read_at(source, &mut buffer[..count], offset, cancel)?;
            output.write_all(&buffer[..count])?;
            offset += count as u64;
        }
        check(cancel)
    })
}
fn read_at(
    source: &File,
    mut bytes: &mut [u8],
    mut offset: u64,
    cancel: &AtomicBool,
) -> io::Result<()> {
    while !bytes.is_empty() {
        check(cancel)?;
        let count = source.seek_read(bytes, offset)?;
        if count == 0 {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "HDR original truncated",
            ));
        }
        offset += count as u64;
        bytes = &mut bytes[count..];
    }
    Ok(())
}
fn check(cancel: &AtomicBool) -> io::Result<()> {
    if cancel.load(Ordering::Acquire) {
        Err(io::Error::new(
            io::ErrorKind::Interrupted,
            "HDR original copy cancelled",
        ))
    } else {
        Ok(())
    }
}

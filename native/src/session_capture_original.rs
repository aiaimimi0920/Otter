//! 内联 HDR 原图恢复；私有 delete-on-close 文件持有字节，逐行验证而不分配完整解码图。
use std::{
    fs::{File, OpenOptions},
    io::{self, BufReader, Read, Seek, SeekFrom, Write},
    os::windows::fs::OpenOptionsExt,
    path::Path,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};
use windows::Win32::{
    Storage::FileSystem::{FILE_ATTRIBUTE_TEMPORARY, FILE_FLAG_DELETE_ON_CLOSE, FILE_SHARE_READ},
    System::Com::CoCreateGuid,
};
pub(crate) fn restore(reference: &str, cancel: &AtomicBool) -> io::Result<Arc<File>> {
    #[cfg(test)]
    let directory = std::path::PathBuf::from("C:/Users/Public/nas_home/AI/GameEditor/linshi");
    #[cfg(not(test))]
    let directory = std::env::temp_dir();
    restore_in(reference, &directory, cancel)
}
fn restore_in(reference: &str, directory: &Path, cancel: &AtomicBool) -> io::Result<Arc<File>> {
    check(cancel)?;
    let encoded = reference
        .strip_prefix("data:image/png;base64,")
        .filter(|value| !value.is_empty())
        .ok_or_else(invalid)?;
    if reference.len() > crate::session_document::MAX_SESSION_BYTES {
        return Err(invalid());
    }
    let id = unsafe { CoCreateGuid() }.map_err(io::Error::other)?;
    // 名字不可由会话指定；create_new 防止覆盖。最后一个句柄关闭即删除本次文件。
    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .create_new(true)
        .share_mode(FILE_SHARE_READ.0)
        .custom_flags(FILE_FLAG_DELETE_ON_CLOSE.0 | FILE_ATTRIBUTE_TEMPORARY.0)
        .open(directory.join(format!("otter-hdr-restore-{id:?}.tmp")))?;
    let mut decoder = base64::read::DecoderReader::new(
        encoded.as_bytes(),
        &base64::engine::general_purpose::STANDARD,
    );
    let mut buffer = [0u8; 48 * 1024];
    let mut total = 0usize;
    loop {
        check(cancel)?;
        let count = decoder.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        total = total.checked_add(count).ok_or_else(invalid)?;
        if total > crate::session_document::MAX_SESSION_BYTES * 3 / 4 {
            return Err(invalid());
        }
        file.write_all(&buffer[..count])?;
    }
    check(cancel)?;
    file.seek(SeekFrom::Start(0))?;
    validate(&mut file, cancel)?;
    check(cancel)?;
    Ok(Arc::new(file))
}
fn validate(file: &mut File, cancel: &AtomicBool) -> io::Result<()> {
    const WORKSPACE: usize = 8 * 1024 * 1024;
    let _budget = crate::pixels::reserve(WORKSPACE + 512 * 1024)
        .ok_or_else(|| io::Error::other("HDR validation budget exhausted"))?;
    let length = file.metadata()?.len();
    let mut input = BufReader::new(file);
    let mut decoder = png::Decoder::new(&mut input);
    decoder.set_limits(png::Limits { bytes: WORKSPACE });
    decoder.set_ignore_text_chunk(true);
    decoder.set_ignore_iccp_chunk(true);
    let mut reader = decoder.read_info().map_err(io::Error::other)?;
    let info = reader.info();
    if info.color_type != png::ColorType::Rgb
        || info.bit_depth != png::BitDepth::Sixteen
        || info.interlaced
        || info.animation_control.is_some()
        || crate::viewport::checked_image_bytes(info.width, info.height).is_none()
        || !info.coding_independent_code_points.is_some_and(|color| {
            color.color_primaries == 9
                && color.transfer_function == 16
                && color.matrix_coefficients == 0
                && color.is_video_full_range_image
        })
    {
        return Err(invalid());
    }
    let (width, height) = (info.width as usize * 6, info.height);
    for _ in 0..height {
        check(cancel)?;
        let row = reader
            .next_row()
            .map_err(io::Error::other)?
            .ok_or_else(invalid)?;
        if row.data().len() != width {
            return Err(invalid());
        }
    }
    if reader.next_row().map_err(io::Error::other)?.is_some() {
        return Err(invalid());
    }
    reader.finish().map_err(io::Error::other)?;
    drop(reader);
    if input.stream_position()? != length {
        return Err(invalid());
    }
    check(cancel)
}
fn invalid() -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, "Invalid inline HDR original")
}
fn check(cancel: &AtomicBool) -> io::Result<()> {
    if cancel.load(Ordering::Acquire) {
        Err(io::Error::new(
            io::ErrorKind::Interrupted,
            "HDR original restore cancelled",
        ))
    } else {
        Ok(())
    }
}
#[cfg(test)]
#[path = "session_capture_original_tests.rs"]
mod tests;

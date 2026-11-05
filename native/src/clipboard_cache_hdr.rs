//! HDR 缓存流式编码与哈希；复用同一配额/排他锁/所有权回执，不缓存整份压缩 PNG。
use super::*;
use sha2::{Digest, Sha256};
use std::io::Write;
pub fn stage(
    profile: &Path,
    stem: &str,
    frame: &crate::capture_color::HdrFrame,
    cancel: &AtomicBool,
    policy: crate::cache_policy::Policy,
) -> io::Result<Prepared> {
    check(cancel)?;
    let raw = crate::viewport::checked_image_bytes(frame.width, frame.height)
        .and_then(|n| (n / 4).checked_mul(6))
        .ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidInput, "Invalid HDR cache dimensions")
        })?;
    if raw != frame.rgb16_be.len() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Invalid HDR cache pixels",
        ));
    }
    // 与 PNG sink 一致的保守上界。提前准入整个上界，不能编码后才发现配额已被其它进程占用。
    let limit = raw as u64 + raw as u64 / 100 + 65536;
    let (directory, lock) = admit_staging(profile, stem, limit, cancel, policy)?;
    let mut draft = None;
    let staged = crate::session_file_write::Staged::prepare_stream(
        &directory.join("pending.png"),
        cancel,
        |file, cancel| {
            let mut sink = Hashed {
                file,
                bytes: 0,
                limit,
                hash: Sha256::new(),
            };
            crate::capture_png::write(&mut sink, frame, cancel).map_err(|error| {
                io::Error::other(format!("HDR cache encoding failed: {error:?}"))
            })?;
            draft = Some(receipt::Draft::from_digest(
                sink.bytes,
                sink.hash.finalize().into(),
            )?);
            Ok(())
        },
    )?;
    Ok(Prepared {
        staged,
        stem: stem.into(),
        receipt: draft.ok_or_else(|| io::Error::other("Missing HDR cache receipt"))?,
        _lock: lock,
    })
}
struct Hashed<'a> {
    file: &'a mut File,
    bytes: u64,
    limit: u64,
    hash: Sha256,
}
impl Write for Hashed<'_> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if self
            .bytes
            .checked_add(bytes.len() as u64)
            .is_none_or(|n| n > self.limit)
        {
            return Err(io::Error::other("HDR cache encoding exceeds admitted size"));
        }
        let count = self.file.write(bytes)?;
        self.hash.update(&bytes[..count]);
        self.bytes += count as u64;
        Ok(count)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.file.flush()
    }
}

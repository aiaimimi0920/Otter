//! HDR PNG 两阶段新文件发布；复用同目录暂存/无覆盖提交，不缓存完整编码文件。
use crate::{capture_color::HdrFrame, session_file_write::Staged};
#[path = "capture_file_original.rs"]
mod original;
pub(crate) use original::encode_inline;
pub(crate) use original::prepare as prepare_original;
use std::{
    io,
    path::{Path, PathBuf},
    sync::atomic::AtomicBool,
};
pub struct PreparedHdr {
    staged: Staged,
}
impl PreparedHdr {
    /// 后台编码及 sync_all；不会创建最终目标。调用者只能为已授权的新文件目的地调用。
    pub fn prepare(destination: &Path, frame: &HdrFrame, cancel: &AtomicBool) -> io::Result<Self> {
        if !destination
            .extension()
            .is_some_and(|value| value.eq_ignore_ascii_case("png"))
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "HDR capture destination must be PNG",
            ));
        }
        let staged = Staged::prepare_stream(destination, cancel, |file, cancel| {
            crate::capture_png::write(file, frame, cancel).map_err(|error| match error {
                crate::capture_png::Error::Cancelled => {
                    io::Error::new(io::ErrorKind::Interrupted, "HDR PNG encoding cancelled")
                }
                error => io::Error::other(format!("HDR PNG encoding failed: {error:?}")),
            })
        })?;
        Ok(Self { staged })
    }
    /// UI owner 必须先复核请求/窗口/文档代次。无覆盖提交成功后不因迟到取消删除文件。
    pub fn commit_new(self, cancel: &AtomicBool) -> io::Result<PathBuf> {
        self.staged.commit_new(cancel)
    }
}
#[cfg(test)]
#[path = "capture_file_tests.rs"]
mod tests;

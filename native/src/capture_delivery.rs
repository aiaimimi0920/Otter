//! 捕获结果的两阶段交付：后台准备原图缓存，UI 身份复核后发布；预览始终来自同帧。
use crate::{
    cache_policy::Policy,
    capture_color::SdrFrame,
    capture_display::DisplayInfo,
    capture_dynamic::{Outcome, Reason},
    clipboard_cache,
};
use std::{
    io,
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
};

pub struct PreparedCapture {
    preview: SdrFrame,
    original: Option<clipboard_cache::Prepared>,
    reason: Option<Reason>,
    downgraded: bool,
    display: Option<DisplayInfo>,
    diagnostic: Option<String>,
}
pub struct Captured {
    pub(crate) backend: &'static str,
    /// 跟随文档而非临时捕获任务存活；不持有缓存全局排他锁。
    pub(crate) original_lease: Option<std::sync::Arc<std::fs::File>>,
    pub preview: SdrFrame,
    /// 仅 HDR 有原图文件；调用方必须将其保留为源，不能用 SDR 预览冒充原图。
    pub original: Option<PathBuf>,
    pub reason: Option<Reason>,
    pub downgraded: bool,
    pub display: Option<DisplayInfo>,
    pub diagnostic: Option<String>,
}
impl PreparedCapture {
    pub(crate) fn prepare(
        outcome: Outcome,
        profile: &Path,
        stem: &str,
        policy: Policy,
        cancel: &AtomicBool,
    ) -> io::Result<Self> {
        check(cancel)?;
        let (preview, hdr) = outcome.frame.into_parts();
        let expected = crate::viewport::checked_image_bytes(preview.width, preview.height);
        if expected != Some(preview.pixels.len())
            || hdr
                .as_ref()
                .is_some_and(|image| (image.width, image.height) != (preview.width, preview.height))
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Invalid capture preview",
            ));
        }
        let original = hdr
            .as_ref()
            .map(|image| clipboard_cache::stage_hdr(profile, stem, image, cancel, policy))
            .transpose()?;
        check(cancel)?;
        // 原始 RGB16 编码完立即释放，只将预览和未发布文件移交 UI，不跨线程保留 GPU 资源。
        Ok(Self {
            preview,
            original,
            reason: outcome.reason,
            downgraded: outcome.downgraded,
            display: outcome.display,
            diagnostic: outcome.diagnostic,
        })
    }
    /// 在 UI 线程调用；accept 须无消息泵地复核窗口、请求和文档代次，失败不发布原图。
    pub fn commit(
        self,
        cancel: &AtomicBool,
        accept: impl FnOnce() -> bool,
    ) -> io::Result<Captured> {
        check(cancel)?;
        if !accept() {
            return Err(io::Error::new(
                io::ErrorKind::Interrupted,
                "Capture owner changed",
            ));
        }
        check(cancel)?;
        let published = self
            .original
            .map(|file| file.commit_leased(cancel))
            .transpose()?;
        let (original, original_lease) = match published {
            Some((path, lease)) => (Some(path), Some(std::sync::Arc::new(lease))),
            None => (None, None),
        };
        // 原图一旦发布就不再因迟到取消删除；与缓存发布的提交点保持一致。
        Ok(Captured {
            backend: if original.is_some() || self.reason == Some(Reason::AutoSdr) {
                "wgc"
            } else {
                "gdi"
            },
            preview: self.preview,
            original,
            original_lease,
            reason: self.reason,
            downgraded: self.downgraded,
            display: self.display,
            diagnostic: self.diagnostic,
        })
    }
}
fn check(cancel: &AtomicBool) -> io::Result<()> {
    if cancel.load(Ordering::Acquire) {
        Err(io::Error::new(
            io::ErrorKind::Interrupted,
            "Capture cancelled",
        ))
    } else {
        Ok(())
    }
}
#[cfg(test)]
#[path = "capture_delivery_tests.rs"]
mod tests;

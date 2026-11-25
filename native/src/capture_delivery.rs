//! Two-phase capture delivery: background original-image caching, then publication after UI identity checks; previews always come from the same frame.
use crate::{
    cache_policy::Policy,
    capture_color::SdrFrame,
    capture_display::DisplayInfo,
    capture_dynamic::{Backend, Outcome, Reason},
    clipboard_cache,
};
use std::{
    io,
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
};

pub struct PreparedCapture {
    backend: Backend,
    preview: SdrFrame,
    original: Option<clipboard_cache::Prepared>,
    reason: Option<Reason>,
    downgraded: bool,
    display: Option<DisplayInfo>,
    diagnostic: Option<String>,
}
pub struct Captured {
    pub(crate) backend: &'static str,
    /// Lives with the document rather than the temporary capture task; does not hold the global exclusive cache lock.
    pub(crate) original_lease: Option<std::sync::Arc<std::fs::File>>,
    pub preview: SdrFrame,
    /// Only HDR has an original-image file; callers must preserve it as the source, never substitute an SDR preview.
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
        // Release original RGB16 immediately after encoding; transfer only the preview and unpublished file to UI, without retaining GPU resources across threads.
        Ok(Self {
            backend: outcome.backend,
            preview,
            original,
            reason: outcome.reason,
            downgraded: outcome.downgraded,
            display: outcome.display,
            diagnostic: outcome.diagnostic,
        })
    }
    /// Call on the UI thread; accept must recheck window/request/document generations without pumping messages, and failure must not publish the original.
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
        // Once published, never delete the original because of late cancellation; match the cache-publication commit point.
        Ok(Captured {
            backend: self.backend.as_str(),
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
#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-live")))]
#[path = "capture_delivery_tests.rs"]
mod tests;

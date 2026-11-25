//! Two-phase publication of new HDR PNG files; reuse same-directory staging/no-overwrite commits without buffering a complete encoded file.
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
    /// Background encoding and sync_all; never creates the final target. Call only for an authorized new-file destination.
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
    /// The UI owner must first recheck request/window/document generations. Never delete a successfully committed no-overwrite file for late cancellation.
    pub fn commit_new(self, cancel: &AtomicBool) -> io::Result<PathBuf> {
        self.staged.commit_new(cancel)
    }
}
#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-capture")))]
#[path = "capture_file_tests.rs"]
mod tests;

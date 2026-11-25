//! Propagate cancellation/layout changes through all error layers; generic SDR fallback must not swallow these terminal signals.
use super::*;
use crate::{capture_readback as readback, capture_wgc as wgc};
pub(super) fn classify(error: capture_target::Error) -> Failure {
    match error {
        capture_target::Error::Cancelled => Failure::Cancelled,
        capture_target::Error::Changed => Failure::Changed,
        capture_target::Error::NoMonitor | capture_target::Error::Api(_) => Failure::Changed,
        capture_target::Error::Capture(wgc::Error::Cancelled) => Failure::Cancelled,
        capture_target::Error::Capture(wgc::Error::Resized) => Failure::Changed,
        capture_target::Error::Capture(wgc::Error::Readback(readback::Error::Cancelled)) => {
            Failure::Cancelled
        }
        capture_target::Error::Capture(wgc::Error::Readback(readback::Error::Conversion(
            capture_color::Error::Cancelled,
        ))) => Failure::Cancelled,
        error => Failure::Retryable(format!("{error:?}")),
    }
}

// Only an actual retryable HDR failure starts another WGC capture. Cancellation and
// a changed target are terminal; even a failed SDR retry must not bypass the target check.
pub(super) fn try_sdr_wgc(
    cancel: &AtomicBool,
    hdr: impl FnOnce() -> Result<Frame, Failure>,
    sdr: impl FnOnce() -> Result<SdrFrame, Failure>,
    validate: impl FnOnce() -> Result<(), Failure>,
) -> Result<WgcFrame, Failure> {
    let hdr_diagnostic = match hdr() {
        Ok(frame) => return Ok(WgcFrame::Native(frame)),
        Err(Failure::Retryable(error)) => error,
        Err(terminal) => return Err(terminal),
    };
    check(cancel).map_err(|_| Failure::Cancelled)?;
    match sdr() {
        Ok(frame) => Ok(WgcFrame::Fallback(frame, hdr_diagnostic)),
        Err(Failure::Retryable(error)) => {
            check(cancel).map_err(|_| Failure::Cancelled)?;
            validate()?;
            Err(Failure::Retryable(format!(
                "HDR WGC: {hdr_diagnostic}; SDR WGC: {error}"
            )))
        }
        Err(terminal) => Err(terminal),
    }
}

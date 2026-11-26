//! Dynamic-range orchestration: one HDR attempt and explainable SDR fallback; cancellation/layout change must never trigger a second capture.
use crate::{
    capture_color::{self, HdrFrame, ScRgb, SdrFrame},
    capture_display::{self, DisplayInfo},
    capture_target::{self, Target},
};
use std::sync::atomic::{AtomicBool, Ordering};
use windows::Win32::Foundation::RECT;
#[path = "capture_dynamic_failure.rs"]
mod failure;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Auto,
    Hdr,
    Sdr,
}
impl Mode {
    /// Match mainline configuration semantics: ignore case and surrounding whitespace, default missing/unknown values to Auto.
    pub fn parse(value: Option<&str>) -> Self {
        match value.map(str::trim) {
            Some(value) if value.eq_ignore_ascii_case("hdr") => Self::Hdr,
            Some(value) if value.eq_ignore_ascii_case("sdr") => Self::Sdr,
            _ => Self::Auto,
        }
    }
}
#[derive(Clone, Copy)]
pub struct Options {
    pub mode: Mode,
    pub standard_region: bool,
    /// Callers supply verified system capability; use false when unconfirmed, never infer from version strings.
    pub windows_11_or_newer: bool,
    pub gain: f32,
}
impl Options {
    /// Callers retain probe-failure diagnostics and decide fallback; never assume an unknown system supports HDR.
    pub fn detect(
        mode: Mode,
        standard_region: bool,
        gain: f32,
    ) -> Result<Self, crate::capture_platform::Error> {
        let windows_11_or_newer = if mode != Mode::Sdr && standard_region {
            crate::capture_platform::query()?.windows_11_or_newer()
        } else {
            false
        };
        Ok(Self {
            mode,
            standard_region,
            windows_11_or_newer,
            gain,
        })
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reason {
    ExplicitSdr,
    Workload,
    CrossMonitor,
    DisplayUnknown,
    DisplaySdr,
    UnsupportedOs,
    HdrFailed,
    AutoSdr,
}
pub enum Frame {
    Sdr(SdrFrame),
    /// The original is for HDR files and preview for the current SDR canvas; both come from the same captured frame.
    Hdr {
        image: HdrFrame,
        preview: SdrFrame,
    },
}
impl Frame {
    /// Transfer, do not copy; callers must preserve the HDR original and never publish the SDR preview as the original.
    pub fn into_parts(self) -> (SdrFrame, Option<HdrFrame>) {
        match self {
            Self::Sdr(preview) => (preview, None),
            Self::Hdr { image, preview } => (preview, Some(image)),
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Backend {
    Wgc,
    Gdi,
}
impl Backend {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Wgc => "wgc",
            Self::Gdi => "gdi",
        }
    }
}
pub struct Outcome {
    pub backend: Backend,
    pub frame: Frame,
    pub reason: Option<Reason>,
    pub downgraded: bool,
    pub display: Option<DisplayInfo>,
    pub diagnostic: Option<String>,
}
#[derive(Debug)]
pub enum Error {
    Dimensions,
    Cancelled,
    Changed,
    Sdr(String),
}
enum WgcFrame {
    Native(Frame),
    Fallback(SdrFrame, String),
}
enum Failure {
    Cancelled,
    Changed,
    Retryable(String),
}
/// Run in a background MTA; sdr may capture only the same screen and cannot publish independently. Callers deliver after success.
pub fn region(
    screen: RECT,
    options: Options,
    cancel: &AtomicBool,
    sdr: impl FnOnce() -> Result<SdrFrame, String>,
) -> Result<Outcome, Error> {
    let size = capture_target::dimensions(screen).map_err(|_| Error::Dimensions)?;
    let result = execute(
        options,
        cancel,
        || {
            let target = Target::resolve(screen).map_err(|error| match error {
                capture_target::Error::CrossMonitor => Reason::CrossMonitor,
                _ => Reason::DisplayUnknown,
            })?;
            let info =
                capture_display::query(target.monitor()).map_err(|_| Reason::DisplayUnknown)?;
            Ok((target, info))
        },
        |target, info| {
            failure::try_sdr_wgc(
                cancel,
                || {
                    let frame = target
                        .capture(cancel, |frame| convert(frame, options, info, cancel))
                        .map_err(failure::classify)?;
                    check(cancel).map_err(|_| Failure::Cancelled)?;
                    // Color state can change even when the monitor rectangle did not move.
                    let current =
                        capture_display::query(target.monitor()).map_err(|_| Failure::Changed)?;
                    if current != info {
                        return Err(Failure::Changed);
                    }
                    Ok(frame)
                },
                || target.capture_sdr(cancel).map_err(failure::classify),
                || target.validate().map_err(|_| Failure::Changed),
            )
        },
        sdr,
    )?;
    validate_output(&result.frame, size)?;
    check(cancel)?;
    Ok(result)
}
fn validate_output(frame: &Frame, size: (u32, u32)) -> Result<(), Error> {
    let (actual, bytes, channels) = match frame {
        Frame::Sdr(frame) => ((frame.width, frame.height), frame.pixels.len(), 4),
        Frame::Hdr { image, preview } => {
            if (preview.width, preview.height) != size
                || crate::viewport::checked_image_bytes(size.0, size.1)
                    != Some(preview.pixels.len())
            {
                return Err(Error::Dimensions);
            }
            ((image.width, image.height), image.rgb16_be.len(), 6)
        }
    };
    let expected = crate::viewport::checked_image_bytes(size.0, size.1)
        .and_then(|n| (n / 4).checked_mul(channels))
        .ok_or(Error::Dimensions)?;
    if actual != size || bytes != expected {
        return Err(Error::Dimensions);
    }
    Ok(())
}
fn convert(
    frame: &ScRgb<'_>,
    options: Options,
    info: DisplayInfo,
    cancel: &AtomicBool,
) -> Result<Frame, capture_color::Error> {
    if options.mode == Mode::Sdr {
        return frame
            .to_sdr(info.sdr_white_nits, options.gain, cancel)
            .map(Frame::Sdr);
    }
    if options.mode == Mode::Auto {
        // Performance audit optimization (Tier 3.3):
        // Fuse the HDR highlight probe directly into the SDR conversion pass via `to_sdr_probing_hdr` so `Mode::Auto`
        // reads the 63–113 MiB D3D11 FP16 staging buffer once instead of twice on ordinary SDR captures, and reuses
        // the already-produced `sdr` preview if HDR highlights are detected.
        let (preview, has_hdr) =
            frame.to_sdr_probing_hdr(info.sdr_white_nits, options.gain, cancel)?;
        return if !has_hdr {
            Ok(Frame::Sdr(preview))
        } else {
            let image = frame.to_hdr(info.minimum_nits, info.maximum_nits, options.gain, cancel)?;
            Ok(Frame::Hdr { image, preview })
        };
    }
    // Generate previews from the original floating-point frame, rather than decoding PQ RGB16 as ordinary sRGB.
    // Each conversion holds its own budget; subsequent failure automatically releases the local preview without delivering partial results.
    let preview = frame.to_sdr(info.sdr_white_nits, options.gain, cancel)?;
    let image = frame.to_hdr(info.minimum_nits, info.maximum_nits, options.gain, cancel)?;
    Ok(Frame::Hdr { image, preview })
}
fn execute<C>(
    options: Options,
    cancel: &AtomicBool,
    probe: impl FnOnce() -> Result<(C, DisplayInfo), Reason>,
    hdr: impl FnOnce(C, DisplayInfo) -> Result<WgcFrame, Failure>,
    sdr: impl FnOnce() -> Result<SdrFrame, String>,
) -> Result<Outcome, Error> {
    check(cancel)?;
    let mut display = None;
    let mut diagnostic = None;
    let (reason, downgraded) = if options.mode == Mode::Sdr {
        (Reason::ExplicitSdr, false)
    } else if !options.standard_region {
        (Reason::Workload, false)
    } else {
        match probe() {
            Err(reason) => (reason, options.mode == Mode::Hdr),
            Ok((target, info)) => {
                display = Some(info);
                check(cancel)?;
                if !info.hdr_enabled {
                    (Reason::DisplaySdr, options.mode == Mode::Hdr)
                } else if !options.windows_11_or_newer {
                    (Reason::UnsupportedOs, true)
                } else {
                    match hdr(target, info) {
                        Ok(result) => {
                            check(cancel)?;
                            let (frame, reason, diagnostic) = match result {
                                WgcFrame::Native(frame) => {
                                    let reason =
                                        matches!(frame, Frame::Sdr(_)).then_some(Reason::AutoSdr);
                                    (frame, reason, None)
                                }
                                WgcFrame::Fallback(frame, error) => {
                                    (Frame::Sdr(frame), Some(Reason::HdrFailed), Some(error))
                                }
                            };
                            return Ok(Outcome {
                                backend: Backend::Wgc,
                                frame,
                                reason,
                                // Match mainline: conversion to SDR after successful HDR-source capture still records fallback, not capture failure.
                                downgraded: reason.is_some(),
                                display,
                                diagnostic,
                            });
                        }
                        Err(Failure::Cancelled) => return Err(Error::Cancelled),
                        Err(Failure::Changed) => return Err(Error::Changed),
                        Err(Failure::Retryable(error)) => diagnostic = Some(error),
                    }
                    (Reason::HdrFailed, true)
                }
            }
        }
    };
    check(cancel)?;
    let result = sdr();
    check(cancel)?;
    let frame = result.map_err(Error::Sdr)?;
    Ok(Outcome {
        backend: Backend::Gdi,
        frame: Frame::Sdr(frame),
        reason: Some(reason),
        downgraded,
        display,
        diagnostic,
    })
}
fn check(cancel: &AtomicBool) -> Result<(), Error> {
    if cancel.load(Ordering::Acquire) {
        Err(Error::Cancelled)
    } else {
        Ok(())
    }
}
#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-capture")))]
#[path = "capture_preview_tests.rs"]
mod preview_tests;
#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-capture")))]
#[path = "capture_dynamic_tests.rs"]
mod tests;

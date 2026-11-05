//! 取消/布局变化贯穿各层错误；这些终止信号不能被通用 SDR fallback 吞掉。
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

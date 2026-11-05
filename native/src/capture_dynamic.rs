//! 动态范围编排：单次 HDR 尝试、可解释 SDR 回退；取消/布局变化不得触发第二次采集。
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
    /// 对齐主干配置语义：大小写与首尾空白不敏感，缺失/未知值使用 Auto。
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
    /// 调用方提供已验证的系统能力；未确认时必须为 false，不从版本字符串猜测。
    pub windows_11_or_newer: bool,
    pub gain: f32,
}
impl Options {
    /// 探测失败由调用方保留诊断并决定降级；不能将未知系统默认为支持 HDR。
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
    /// 原图用于 HDR 文件，preview 供当前 SDR 画布使用；二者来自同一捕获帧。
    Hdr {
        image: HdrFrame,
        preview: SdrFrame,
    },
}
impl Frame {
    /// 移交而非复制；调用方必须保留 HDR 原图，不能将 SDR 预览当作原图发布。
    pub fn into_parts(self) -> (SdrFrame, Option<HdrFrame>) {
        match self {
            Self::Sdr(preview) => (preview, None),
            Self::Hdr { image, preview } => (preview, Some(image)),
        }
    }
}
pub struct Outcome {
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
enum Failure {
    Cancelled,
    Changed,
    Retryable(String),
}
/// 在后台 MTA 执行；sdr 只能采集相同 screen，不得自行发布。成功后再由调用方交付。
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
            let frame = target
                .capture(cancel, |frame| convert(frame, options, info, cancel))
                .map_err(failure::classify)?;
            if cancel.load(Ordering::Acquire) {
                return Err(Failure::Cancelled);
            }
            // HDR 开关/白电平可能变化而显示器矩形不变；不交付使用旧颜色配置转换的结果。
            let current = capture_display::query(target.monitor()).map_err(|_| Failure::Changed)?;
            if current != info {
                return Err(Failure::Changed);
            }
            Ok(frame)
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
    if options.mode == Mode::Sdr
        || (options.mode == Mode::Auto
            && !capture_color::exceeds_sdr_white(
                frame.analyze(options.gain, cancel)?,
                info.sdr_white_nits,
            ))
    {
        frame
            .to_sdr(info.sdr_white_nits, options.gain, cancel)
            .map(Frame::Sdr)
    } else {
        // 从原始浮点帧生成预览，而不是把 PQ RGB16 当作普通 sRGB 解码。
        // 两次转换各持预算；后续失败时局部 preview 自动释放，不交付半成品。
        let preview = frame.to_sdr(info.sdr_white_nits, options.gain, cancel)?;
        let image = frame.to_hdr(info.minimum_nits, info.maximum_nits, options.gain, cancel)?;
        Ok(Frame::Hdr { image, preview })
    }
}
fn execute<C>(
    options: Options,
    cancel: &AtomicBool,
    probe: impl FnOnce() -> Result<(C, DisplayInfo), Reason>,
    hdr: impl FnOnce(C, DisplayInfo) -> Result<Frame, Failure>,
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
                        Ok(frame) => {
                            check(cancel)?;
                            let reason = matches!(frame, Frame::Sdr(_)).then_some(Reason::AutoSdr);
                            return Ok(Outcome {
                                frame,
                                reason,
                                // 对齐主干：HDR 源成功采集后转为 SDR 仍须记录降级，非采集失败。
                                downgraded: reason == Some(Reason::AutoSdr),
                                display,
                                diagnostic: None,
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
#[cfg(test)]
#[path = "capture_preview_tests.rs"]
mod preview_tests;
#[cfg(test)]
#[path = "capture_dynamic_tests.rs"]
mod tests;

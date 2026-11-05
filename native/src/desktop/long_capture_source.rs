//! 显式选区后台采集；手动和绑定目标的单步滚动分开准入，不控制焦点。
use crate::{
    long_capture::{Axis, CaptureMode, Frame, MatchOptions, Session, SessionJob},
    pixels::Pixels,
};
use std::sync::atomic::Ordering;
#[path = "long_capture_source_owner.rs"]
mod owner;
#[path = "long_capture_scroll.rs"]
pub mod scroll;
#[path = "long_capture_stable.rs"]
mod stable;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Region {
    pub left: i32,
    pub top: i32,
    pub width: i32,
    pub height: i32,
}
/// first 来自已确认选区，后续 capture 命令由同一 worker 截取该物理矩形并追加。
/// 应用须先完成已有 PER_MONITOR_AWARE_V2 初始化；屏幕布局变化须由宿主停止会话。
pub fn start(
    region: Region,
    first: Pixels,
    axis: Axis,
    options: MatchOptions,
) -> Result<SessionJob, String> {
    let session = prepare(region, first, axis, options)?;
    SessionJob::start_with_source(session, owner::Source::new(region, axis, None))
        .map_err(|error| error.to_string())
}
fn prepare(
    region: Region,
    first: Pixels,
    axis: Axis,
    options: MatchOptions,
) -> Result<Session, String> {
    super::capture::validate_region(region.left, region.top, region.width, region.height)
        .map_err(|error| error.to_string())?;
    let frame = Frame::new(region.width as u32, region.height as u32, first)
        .map_err(|error| format!("Invalid long capture frame: {error:?}"))?;
    Session::new(axis, frame, options)
        .map_err(|error| format!("Invalid long capture options: {error:?}"))
}
fn capture(region: Region, cancel: &std::sync::atomic::AtomicBool) -> Result<Frame, String> {
    if cancel.load(Ordering::Acquire) {
        return Err("Long capture cancelled".into());
    }
    // GDI 调用本身不可中断；返回后先复核取消，避免发布迟到图像。
    let image = super::capture::region(region.left, region.top, region.width, region.height)
        .map_err(|error| error.to_string())?;
    if cancel.load(Ordering::Acquire) {
        return Err("Long capture cancelled".into());
    }
    Frame::new(image.width, image.height, image.pixels)
        .map_err(|error| format!("Invalid captured frame: {error:?}"))
}
/// 绑定发生在控制窗口显示之前；绑定失败由 UI 保留手动模式，不伪装支持自动滚动。
pub fn start_scrollable(
    region: Region,
    first: Pixels,
    axis: Axis,
    options: MatchOptions,
) -> Result<SessionJob, String> {
    let session = prepare(region, first, axis, options)?;
    let target = scroll::Target::bind(region)?;
    SessionJob::start_with_source(session, owner::Source::new(region, axis, Some(target)))
        .map_err(|error| error.to_string())
}
/// 在控制窗口出现前冻结目标。观察源没有主动发送输入的能力，旧主动采集入口不受影响。
pub fn start_recording(
    region: Region,
    first: Pixels,
) -> Result<(SessionJob, scroll::Target), String> {
    super::capture::validate_region(region.left, region.top, region.width, region.height)
        .map_err(|error| error.to_string())?;
    let first = Frame::new(region.width as u32, region.height as u32, first)
        .map_err(|error| format!("Invalid recording frame: {error:?}"))?;
    let target = scroll::Target::bind(region)?;
    let source = owner::Source::recording(region, target.clone());
    let job = SessionJob::start_detecting(first, source).map_err(|error| error.to_string())?;
    Ok((job, target))
}
// 稳定采样最多同时保留上一帧 BGRA、当前 DIB 和当前 BGRA。发送输入前先检查峰值，
// 不长期占用租约；实际 GDI 分配仍逐项执行预算准入，防止检查后其它文档发生分配竞争。
fn capture_headroom(region: Region) -> Result<(), String> {
    let bytes = u64::try_from(region.width)
        .ok()
        .and_then(|width| {
            u64::try_from(region.height)
                .ok()
                .and_then(|height| width.checked_mul(height))
        })
        .and_then(|pixels| pixels.checked_mul(12))
        .and_then(|bytes| usize::try_from(bytes).ok())
        .filter(|bytes| *bytes > 0)
        .ok_or("Invalid capture allocation dimensions")?;
    let _headroom = crate::pixels::reserve(bytes)
        .ok_or("Insufficient pixel budget for capture stabilization")?;
    Ok(())
}
fn settle(cancel: &std::sync::atomic::AtomicBool) -> Result<(), String> {
    wait(cancel, std::time::Duration::from_millis(250))
}
fn wait(
    cancel: &std::sync::atomic::AtomicBool,
    duration: std::time::Duration,
) -> Result<(), String> {
    let deadline = std::time::Instant::now() + duration;
    loop {
        if cancel.load(Ordering::Acquire) {
            return Err("Long capture cancelled while settling".into());
        }
        let remaining = deadline.saturating_duration_since(std::time::Instant::now());
        if remaining.is_zero() {
            return Ok(());
        }
        std::thread::sleep(remaining.min(std::time::Duration::from_millis(10)));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn capture_headroom_checks_dimensions_without_allocating_or_capturing() {
        for (width, height) in [(0, 2), (-1, 2), (2, -1), (i32::MAX, i32::MAX)] {
            assert!(capture_headroom(Region {
                left: 0,
                top: 0,
                width,
                height
            })
            .is_err());
        }
        assert!(capture_headroom(Region {
            left: -10,
            top: -20,
            width: 2,
            height: 2
        })
        .is_ok());
    }
    #[test]
    fn cancelled_settle_does_not_continue_to_capture() {
        assert!(settle(&std::sync::atomic::AtomicBool::new(true)).is_err());
    }
    #[test]
    fn invalid_region_and_frame_fail_without_starting_capture() {
        let options = MatchOptions {
            minimum_overlap: 1,
            maximum_shift: 1,
            maximum_mean_error: 3.0,
        };
        for region in [
            Region {
                left: i32::MAX,
                top: 0,
                width: 2,
                height: 2,
            },
            Region {
                left: 0,
                top: 0,
                width: 0,
                height: 2,
            },
        ] {
            assert!(start(region, vec![0; 16].into(), Axis::Vertical, options).is_err());
        }
        let region = Region {
            left: -10,
            top: -20,
            width: 2,
            height: 2,
        };
        assert!(start(region, vec![0; 4].into(), Axis::Vertical, options).is_err());
        // 合法构造也不采集：没有显式 capture 命令，worker 只等待通道。
        let mut job = start(region, vec![0; 16].into(), Axis::Vertical, options).unwrap();
        job.cancel();
    }
}

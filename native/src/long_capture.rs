//! 长截图的已定位帧拼接底层；位置由匹配器提供，本模块不猜测滚动、不执行屏幕采集。
use crate::pixels::Pixels;
use std::sync::atomic::AtomicBool;
#[path = "long_capture_compose.rs"]
mod compose;
#[path = "long_capture_match.rs"]
mod matching;
#[path = "long_capture_view.rs"]
mod view;
pub use matching::{match_exact, Match};
#[path = "long_capture_approx.rs"]
mod approximate;
pub use approximate::{match_approximate, ApproximateMatch, MatchOptions};
#[path = "long_capture_fixed.rs"]
mod fixed;
pub use fixed::{match_fixed_edges, stitch_fixed_edges, FixedMatch};
#[path = "long_capture_session.rs"]
mod session;
pub use session::{Append, Session};
#[path = "long_capture_job.rs"]
mod job;
pub use job::{CaptureMode, CaptureSource, Completion, DispatchError, Reply, SessionJob, Snapshot};
#[path = "long_capture_spatial.rs"]
mod spatial;
pub use spatial::{stitch_spatial, SpatialComposite, SpatialFrame};
#[path = "long_capture_spatial_match.rs"]
mod spatial_matching;
pub use spatial_matching::{match_spatial, SpatialMatch};
#[path = "long_capture_spatial_session.rs"]
mod spatial_session;
pub use spatial_session::{SpatialAppend, SpatialSession};
pub const MAX_FRAMES: usize = 512;
const MAX_FRAME_AXIS: u32 = 32_768;
const MAX_FRAME_PIXELS: u64 = 64 * 1024 * 1024;
const MAX_INPUT_PIXELS: u64 = 128 * 1024 * 1024;
const MAX_OUTPUT_AXIS: u32 = 131_072;
const MAX_OUTPUT_PIXELS: u64 = 128 * 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Axis {
    Vertical,
    Horizontal,
}
/// 已接受图像在拼接坐标中的运动方向；不是滚轮符号，也不由输入事件直接赋值。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}
impl Direction {
    pub(crate) fn from_offset(axis: Axis, delta: i64) -> Option<Self> {
        match (axis, delta.signum()) {
            (Axis::Vertical, -1) => Some(Self::Up),
            (Axis::Vertical, 1) => Some(Self::Down),
            (Axis::Horizontal, -1) => Some(Self::Left),
            (Axis::Horizontal, 1) => Some(Self::Right),
            _ => None,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Empty,
    FrameLimit,
    Dimensions,
    Pixels,
    InputLimit,
    CrossAxis,
    Gap,
    OutputLimit,
    Budget,
    Cancelled,
    ChangedEdges,
    UnconfirmedEdges,
}
#[derive(Clone, Debug)]
pub struct Frame {
    width: u32,
    height: u32,
    pixels: Pixels,
}
impl Frame {
    /// 紧密排列的 BGRA 像素；克隆 Frame 只共享像素和预算租约。
    pub fn new(width: u32, height: u32, pixels: Pixels) -> Result<Self, Error> {
        let count = frame_pixels(width, height)?;
        if count * 4 != pixels.len() as u64 {
            return Err(Error::Pixels);
        }
        Ok(Self {
            width,
            height,
            pixels,
        })
    }
    pub fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }
    pub fn bytes(&self) -> &[u8] {
        &self.pixels
    }
}
/// 滚动轴上的绝对位置；允许负坐标，首帧不要求在零点。
pub struct Positioned {
    pub frame: Frame,
    pub origin: i64,
}
pub struct Composite {
    pub width: u32,
    pub height: u32,
    pub origin: i64,
    pub pixels: Pixels,
}

/// 保留较早帧的已覆盖内容，只补入新区域；不进行 alpha 混合或隐式颜色转换。
/// 空洞、横向尺寸不一致及超限在输出分配前拒绝；共享像素预算可能早于单图上限拒绝。
pub fn stitch(axis: Axis, frames: &[Positioned], cancel: &AtomicBool) -> Result<Composite, Error> {
    compose::stitch(axis, frames, cancel, crate::pixels::reserve)
}
fn frame_pixels(width: u32, height: u32) -> Result<u64, Error> {
    if width == 0 || height == 0 || width > MAX_FRAME_AXIS || height > MAX_FRAME_AXIS {
        return Err(Error::Dimensions);
    }
    let count = u64::from(width) * u64::from(height);
    if count > MAX_FRAME_PIXELS {
        return Err(Error::Dimensions);
    }
    Ok(count)
}
fn output_bytes(width: u32, height: u32) -> Result<usize, Error> {
    let count = u64::from(width) * u64::from(height);
    if width == 0
        || height == 0
        || width > MAX_OUTPUT_AXIS
        || height > MAX_OUTPUT_AXIS
        || count > MAX_OUTPUT_PIXELS
    {
        return Err(Error::OutputLimit);
    }
    usize::try_from(count * 4).map_err(|_| Error::OutputLimit)
}

#[cfg(test)]
#[path = "long_capture_tests.rs"]
mod tests;

//! 二维采集会话的单所有者状态；先验证再提交，错误不会改变最后接受帧或累计坐标。
use super::*;
use std::sync::atomic::Ordering;
#[path = "long_capture_spatial_preview.rs"]
mod preview;
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SpatialAppend {
    Added { x: i64, y: i64 },
    Duplicate,
    Rejected(ApproximateMatch),
}
pub struct SpatialSession {
    frames: Vec<SpatialFrame>,
    reference: Frame,
    cursor: (i64, i64),
    accepted_count: usize,
    input_total: u64,
    axis: Axis,
    options: MatchOptions,
}
impl SpatialSession {
    pub fn new(axis: Axis, first: Frame, options: MatchOptions) -> Result<Self, Error> {
        // 与一维会话共用参数准入，克隆仅增加共享像素引用。
        Session::new(axis, first.clone(), options)?;
        Ok(Self {
            reference: first.clone(),
            cursor: (0, 0),
            accepted_count: 1,
            input_total: u64::from(first.width) * u64::from(first.height),
            frames: vec![SpatialFrame {
                frame: first,
                x: 0,
                y: 0,
            }],
            axis,
            options,
        })
    }
    pub fn frame_count(&self) -> usize {
        self.accepted_count
    }
    pub fn axis(&self) -> Axis {
        self.axis
    }
    pub fn position(&self) -> (i64, i64) {
        self.cursor
    }
    pub(super) fn admit_next(&self) -> Result<(), Error> {
        if self.accepted_count >= MAX_FRAMES {
            return Err(Error::FrameLimit);
        }
        if self.input_total + u64::from(self.reference.width) * u64::from(self.reference.height)
            > MAX_INPUT_PIXELS
        {
            return Err(Error::InputLimit);
        }
        let (_, _, mut width, mut height) = spatial::bounds(&self.frames, &AtomicBool::new(false))?;
        let length = match self.axis {
            Axis::Vertical => self.reference.height,
            Axis::Horizontal => self.reference.width,
        };
        let shift = self
            .options
            .maximum_shift
            .min(length - self.options.minimum_overlap);
        match self.axis {
            Axis::Vertical => height = height.checked_add(shift).ok_or(Error::OutputLimit)?,
            Axis::Horizontal => width = width.checked_add(shift).ok_or(Error::OutputLimit)?,
        }
        if u64::from(width) * u64::from(height) > 64 * 1024 * 1024 {
            return Err(Error::OutputLimit);
        }
        let bytes = output_bytes(width, height)?
            .checked_add(self.reference.pixels.len())
            .ok_or(Error::Budget)?;
        let _headroom = crate::pixels::reserve(bytes).ok_or(Error::Budget)?;
        Ok(())
    }
    pub(super) fn from_capture(
        axis: Axis,
        options: MatchOptions,
        aggregate: Composite,
        reference: Frame,
        cursor: (i64, i64),
        accepted_count: usize,
        input_total: u64,
    ) -> Result<Self, Error> {
        // 聚合图不是新的截图帧：允许超过单帧轴长，但仍受输出与二维 64 MP 上限约束。
        let expected = output_bytes(aggregate.width, aggregate.height)?;
        if aggregate.pixels.len() != expected {
            return Err(Error::Pixels);
        }
        let (x, y) = match axis {
            Axis::Vertical => (0, aggregate.origin),
            Axis::Horizontal => (aggregate.origin, 0),
        };
        let frames = vec![SpatialFrame {
            x,
            y,
            frame: Frame {
                width: aggregate.width,
                height: aggregate.height,
                pixels: aggregate.pixels,
            },
        }];
        spatial::bounds(&frames, &AtomicBool::new(false))?;
        Ok(Self {
            frames,
            reference,
            cursor,
            accepted_count,
            input_total,
            axis,
            options,
        })
    }
    /// movement 是相对于最后接受帧的物理取景框位移；失败后的下一次请求仍须用该基准。
    pub fn append(
        &mut self,
        frame: Frame,
        movement: (i64, i64),
        cancel: &AtomicBool,
    ) -> Result<SpatialAppend, Error> {
        self.append_with_reserve(frame, movement, cancel, crate::pixels::reserve)
    }
    fn append_with_reserve(
        &mut self,
        frame: Frame,
        movement: (i64, i64),
        cancel: &AtomicBool,
        reserve: impl FnOnce(usize) -> Option<crate::byte_budget::Lease>,
    ) -> Result<SpatialAppend, Error> {
        check(cancel)?;
        let (dx, dy) = match match_spatial(
            &self.reference,
            &frame,
            movement,
            self.axis,
            self.options,
            cancel,
        )? {
            SpatialMatch::Rejected(reason) => return Ok(SpatialAppend::Rejected(reason)),
            SpatialMatch::Offset { x, y, .. } => (x, y),
        };
        if dx == 0 && dy == 0 && frame.bytes() == self.reference.bytes() {
            check(cancel)?;
            return Ok(SpatialAppend::Duplicate);
        }
        let x = self.cursor.0.checked_add(dx).ok_or(Error::OutputLimit)?;
        let y = self.cursor.1.checked_add(dy).ok_or(Error::OutputLimit)?;
        if self.accepted_count >= MAX_FRAMES {
            return Err(Error::FrameLimit);
        }
        let total = self.input_total + u64::from(frame.width) * u64::from(frame.height);
        if total > MAX_INPUT_PIXELS {
            return Err(Error::InputLimit);
        }
        self.frames.try_reserve(1).map_err(|_| Error::Budget)?;
        let reference = frame.clone();
        self.frames.push(SpatialFrame { frame, x, y });
        // 临时候选仅在独占 &mut 内可见，所有失败分支统一弹出；此后才算接受帧。
        let admitted = (|| {
            let (_, _, width, height) = spatial::bounds(&self.frames, cancel)?;
            let _headroom = reserve(output_bytes(width, height)?).ok_or(Error::Budget)?;
            check(cancel)
        })();
        if let Err(error) = admitted {
            self.frames.pop();
            return Err(error);
        }
        self.reference = reference;
        self.cursor = (x, y);
        self.accepted_count += 1;
        self.input_total = total;
        Ok(SpatialAppend::Added { x, y })
    }
    pub fn compose(&self, cancel: &AtomicBool) -> Result<SpatialComposite, Error> {
        stitch_spatial(&self.frames, cancel)
    }
    /// 预览包含透明空洞和完整覆盖范围，不按最终导出的最大矩形裁剪。
    pub fn preview(&self, cancel: &AtomicBool) -> Result<SpatialComposite, Error> {
        preview::render(&self.frames, cancel)
    }
}
fn check(cancel: &AtomicBool) -> Result<(), Error> {
    if cancel.load(Ordering::Acquire) {
        Err(Error::Cancelled)
    } else {
        Ok(())
    }
}
#[cfg(test)]
#[path = "long_capture_spatial_session_tests.rs"]
mod tests;

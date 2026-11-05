//! 纯数据采集会话；持有共享帧，追加先验证再提交，不负责桌面输入或线程调度。
use super::*;
use std::sync::atomic::Ordering;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Append {
    Added { origin: i64 },
    Duplicate,
    Rejected(ApproximateMatch),
}
pub struct Session {
    axis: Axis,
    options: MatchOptions,
    frames: Vec<Positioned>,
    edges: Option<(u32, u32)>,
    confirmations: u8,
    total: u64,
    minimum: i64,
    maximum: i64,
    motion: Option<i64>,
}
impl Session {
    pub fn new(axis: Axis, first: Frame, options: MatchOptions) -> Result<Self, Error> {
        let length = axis_length(axis, &first);
        if options.minimum_overlap == 0
            || options.minimum_overlap > length
            || !options.maximum_mean_error.is_finite()
            || !(0.0..=12.0).contains(&options.maximum_mean_error)
        {
            return Err(Error::Dimensions);
        }
        let total = u64::from(first.width) * u64::from(first.height);
        Ok(Self {
            axis,
            options,
            frames: vec![Positioned {
                frame: first,
                origin: 0,
            }],
            edges: None,
            confirmations: 0,
            total,
            minimum: 0,
            maximum: 0,
            motion: None,
        })
    }
    pub fn frame_count(&self) -> usize {
        self.frames.len()
    }
    pub fn axis(&self) -> Axis {
        self.axis
    }
    /// 最近已提交的图像位移；重复、拒绝、取消及预算失败都不能改变方向证据。
    pub fn motion(&self) -> Option<i64> {
        self.motion
    }
    // 滚动前先检查已知的帧数和逻辑输入上限，不先改变外部应用再发现无法保留下一帧。
    pub(super) fn admit_next(&self) -> Result<(), Error> {
        if self.frames.len() >= MAX_FRAMES {
            return Err(Error::FrameLimit);
        }
        let first = &self.frames[0].frame;
        if self.total + u64::from(first.width) * u64::from(first.height) > MAX_INPUT_PIXELS {
            return Err(Error::InputLimit);
        }
        // 下次位移尚未知，滚动前按允许的最大扩展检查最终输出和下一帧的同时驻留。
        let length = axis_length(self.axis, first);
        let shift = self
            .options
            .maximum_shift
            .min(length - self.options.minimum_overlap);
        let span = self
            .maximum
            .checked_sub(self.minimum)
            .and_then(|n| n.checked_add(i64::from(length) + i64::from(shift)))
            .and_then(|n| u32::try_from(n).ok())
            .ok_or(Error::OutputLimit)?;
        let (width, height) = match self.axis {
            Axis::Vertical => (first.width, span),
            Axis::Horizontal => (span, first.height),
        };
        let bytes = output_bytes(width, height)?
            .checked_add(first.pixels.len())
            .ok_or(Error::Budget)?;
        let _headroom = crate::pixels::reserve(bytes).ok_or(Error::Budget)?;
        Ok(())
    }
    /// 至少两次非零运动观察到一致固定边缘才确认；零边缘无需等待第三帧。
    pub fn confirmed_edges(&self) -> Option<(u32, u32)> {
        self.edges
            .filter(|edges| *edges == (0, 0) || self.confirmations >= 2)
    }
    pub fn append(&mut self, frame: Frame, cancel: &AtomicBool) -> Result<Append, Error> {
        self.append_with_reserve(frame, cancel, crate::pixels::reserve)
    }
    fn append_with_reserve(
        &mut self,
        frame: Frame,
        cancel: &AtomicBool,
        reserve: impl FnOnce(usize) -> Option<crate::byte_budget::Lease>,
    ) -> Result<Append, Error> {
        check(cancel)?;
        let previous = self
            .frames
            .last()
            .expect("session always has a first frame");
        if previous.frame.dimensions() != frame.dimensions() {
            return Err(Error::Dimensions);
        }
        if let Some((leading, trailing)) = self.edges {
            let length = axis_length(self.axis, &frame);
            for line in (0..leading).chain(length - trailing..length) {
                check(cancel)?;
                if !fixed::equal_line(&self.frames[0].frame, &frame, self.axis, line) {
                    return Err(Error::ChangedEdges);
                }
            }
        }
        // 精确重复不计入帧数或输入预算，不改变最后接受帧和边界证据。
        let found = if self.edges.is_none_or(|edges| edges == (0, 0)) {
            match match_exact(
                &previous.frame,
                &frame,
                self.axis,
                self.options.minimum_overlap,
                cancel,
            )? {
                Match::Duplicate => FixedMatch::Unchanged,
                Match::Offset(delta)
                    if delta.unsigned_abs() <= u64::from(self.options.maximum_shift) =>
                {
                    FixedMatch::Motion {
                        leading: 0,
                        trailing: 0,
                        delta,
                        mean_error: 0.0,
                    }
                }
                Match::Offset(_) => FixedMatch::Unresolved(ApproximateMatch::NoOverlap),
                Match::Ambiguous => FixedMatch::Unresolved(ApproximateMatch::Ambiguous),
                Match::NoOverlap => {
                    match_fixed_edges(&previous.frame, &frame, self.axis, self.options, cancel)?
                }
            }
        } else {
            match_fixed_edges(&previous.frame, &frame, self.axis, self.options, cancel)?
        };
        let (edges, delta) = match found {
            FixedMatch::Unchanged => return Ok(Append::Duplicate),
            FixedMatch::Unresolved(result) => return Ok(Append::Rejected(result)),
            FixedMatch::Motion {
                leading,
                trailing,
                delta,
                ..
            } => ((leading, trailing), delta),
        };
        if self.edges.is_some_and(|old| old != edges) {
            return Err(Error::ChangedEdges);
        }
        if self.frames.len() >= MAX_FRAMES {
            return Err(Error::FrameLimit);
        }
        let total = self.total + u64::from(frame.width) * u64::from(frame.height);
        if total > MAX_INPUT_PIXELS {
            return Err(Error::InputLimit);
        }
        let origin = previous
            .origin
            .checked_add(delta)
            .ok_or(Error::OutputLimit)?;
        let minimum = self.minimum.min(origin);
        let maximum = self.maximum.max(origin);
        let length = axis_length(self.axis, &frame);
        let span = maximum
            .checked_sub(minimum)
            .and_then(|n| n.checked_add(i64::from(length)))
            .and_then(|n| u32::try_from(n).ok())
            .ok_or(Error::OutputLimit)?;
        minimum
            .checked_sub(i64::from(edges.0))
            .ok_or(Error::OutputLimit)?;
        let (width, height) = match self.axis {
            Axis::Vertical => (frame.width, span),
            Axis::Horizontal => (span, frame.height),
        };
        // 候选像素已计入共享预算，再确认完整输出可同时驻留。这里只取得短租约，不分配长图。
        // 租约随本次准入结束释放；后续其它文档仍可能消耗预算，合成必须再次检查。
        let _output_headroom = reserve(output_bytes(width, height)?).ok_or(Error::Budget)?;
        check(cancel)?;
        self.frames.try_reserve(1).map_err(|_| Error::Budget)?;
        self.frames.push(Positioned { frame, origin });
        self.edges = Some(edges);
        self.confirmations = self.confirmations.saturating_add(1);
        self.total = total;
        self.minimum = minimum;
        self.maximum = maximum;
        self.motion = Some(delta);
        Ok(Append::Added { origin })
    }
    /// 不消费会话：输出预算失败或取消后可重试；Drop 会释放所有输入帧引用。
    pub fn compose(&self, cancel: &AtomicBool) -> Result<Composite, Error> {
        check(cancel)?;
        if self.frames.len() == 1 {
            return stitch(self.axis, &self.frames, cancel);
        }
        let edges = self.confirmed_edges().ok_or(Error::UnconfirmedEdges)?;
        stitch_fixed_edges(self.axis, &self.frames, edges.0, edges.1, cancel)
    }
    pub fn preview(&self, cancel: &AtomicBool) -> Result<Composite, Error> {
        check(cancel)?;
        let edges = if self.frames.len() == 1 {
            (0, 0)
        } else {
            self.confirmed_edges().ok_or(Error::UnconfirmedEdges)?
        };
        compose::preview(self.axis, &self.frames, edges, cancel)
    }
    /// 生成独立候选，不消费当前会话；调用方只在后续移动帧也成功接受后替换旧会话。
    pub fn to_spatial(&self, cancel: &AtomicBool) -> Result<SpatialSession, Error> {
        let aggregate = self.compose(cancel)?;
        let last = self
            .frames
            .last()
            .expect("session always has a first frame");
        let leading = self.confirmed_edges().map_or(0, |edges| edges.0);
        let origin = last
            .origin
            .checked_sub(i64::from(leading))
            .ok_or(Error::OutputLimit)?;
        let cursor = match self.axis {
            Axis::Vertical => (0, origin),
            Axis::Horizontal => (origin, 0),
        };
        check(cancel)?;
        let converted = SpatialSession::from_capture(
            self.axis,
            self.options,
            aggregate,
            last.frame.clone(),
            cursor,
            self.frames.len(),
            self.total,
        )?;
        check(cancel)?;
        Ok(converted)
    }
}
fn axis_length(axis: Axis, frame: &Frame) -> u32 {
    match axis {
        Axis::Vertical => frame.height,
        Axis::Horizontal => frame.width,
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
#[path = "long_capture_session_tests.rs"]
mod tests;

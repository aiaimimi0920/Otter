//! worker 持有采集区域与候选；候选截图失败/取消不更新区域或滚动目标。
use super::*;
use crate::long_capture::CaptureSource;
use std::sync::atomic::AtomicBool;
pub(super) struct Source {
    region: Region,
    axis: Axis,
    target: Option<scroll::Target>,
    pending: Option<(Region, Option<scroll::Target>)>,
    remaining: u32,
    layout: Option<(i32, i32, i32, i32, i32)>,
}
impl Source {
    pub fn new(region: Region, axis: Axis, target: Option<scroll::Target>) -> Self {
        Self {
            region,
            axis,
            target,
            pending: None,
            remaining: 511,
            layout: None,
        }
    }
    pub fn recording(region: Region, target: scroll::Target) -> Self {
        Self {
            layout: Some(layout()),
            ..Self::new(region, Axis::Vertical, Some(target))
        }
    }
    fn validate(&self) -> Result<(), String> {
        if self.layout.is_some_and(|expected| expected != layout()) {
            return Err("Screen layout changed during recording".into());
        }
        if let Some(target) = &self.target {
            target.validate()?;
        }
        Ok(())
    }
    fn prepare_with(
        &mut self,
        movement: (i32, i32),
        cancel: &AtomicBool,
        desktop: Region,
        sample: impl FnOnce(Region, Option<&scroll::Target>, &AtomicBool) -> Result<Frame, String>,
    ) -> Result<Frame, String> {
        self.pending = None;
        check(cancel)?;
        self.validate()?;
        let candidate = translated(self.region, movement, desktop)?;
        let target = self
            .target
            .as_ref()
            .map(|target| target.relocate(candidate))
            .transpose()?
            .flatten();
        capture_headroom(candidate)?;
        let frame = sample(candidate, target.as_ref(), cancel)?;
        check(cancel)?;
        self.validate()?;
        if frame.dimensions() != (candidate.width as u32, candidate.height as u32) {
            return Err("Moved capture dimensions changed".into());
        }
        // 降级手动也要复核旧目标；不能用离开窗口绕过关闭/移动/身份变化。
        if let Some(target) = &self.target {
            target.validate()?;
        }
        self.pending = Some((candidate, target));
        Ok(frame)
    }
}
impl CaptureSource for Source {
    fn can_scroll(&self) -> bool {
        self.target.is_some() && self.layout.is_none()
    }
    fn can_move(&self) -> bool {
        true
    }
    fn capture(&mut self, mode: CaptureMode, cancel: &AtomicBool) -> Result<Frame, String> {
        check(cancel)?;
        self.validate()?;
        if mode == CaptureMode::Current {
            capture_headroom(self.region)?;
            let frame = capture(self.region, cancel)?;
            self.validate()?;
            return Ok(frame);
        }
        if self.layout.is_some() {
            return Err("Recording source does not send input".into());
        }
        let target = self.target.as_ref().ok_or("Scroll source unavailable")?;
        if self.remaining == 0 {
            return Err("Long capture scroll attempt limit reached".into());
        }
        capture_headroom(self.region)?;
        self.remaining -= 1; // 已交付但随后失败的输入也计数，不自动重发。
        target.step(self.axis, mode == CaptureMode::ScrollForward, cancel)?;
        settle(cancel)?;
        sample(self.region, Some(target), cancel)
    }
    fn prepare_move(&mut self, movement: (i32, i32), cancel: &AtomicBool) -> Result<Frame, String> {
        use windows::Win32::UI::WindowsAndMessaging::*;
        let desktop = unsafe {
            Region {
                left: GetSystemMetrics(SM_XVIRTUALSCREEN),
                top: GetSystemMetrics(SM_YVIRTUALSCREEN),
                width: GetSystemMetrics(SM_CXVIRTUALSCREEN),
                height: GetSystemMetrics(SM_CYVIRTUALSCREEN),
            }
        };
        self.prepare_with(movement, cancel, desktop, sample)
    }
    fn finish_move(&mut self, accepted: bool) {
        if let Some((region, target)) = self.pending.take() {
            if accepted {
                self.region = region;
                self.target = target;
            }
        }
    }
}
fn layout() -> (i32, i32, i32, i32, i32) {
    use windows::Win32::UI::WindowsAndMessaging::*;
    unsafe {
        (
            GetSystemMetrics(SM_XVIRTUALSCREEN),
            GetSystemMetrics(SM_YVIRTUALSCREEN),
            GetSystemMetrics(SM_CXVIRTUALSCREEN),
            GetSystemMetrics(SM_CYVIRTUALSCREEN),
            GetSystemMetrics(SM_CMONITORS),
        )
    }
}
fn check(cancel: &AtomicBool) -> Result<(), String> {
    if cancel.load(Ordering::Acquire) {
        Err("Long capture cancelled".into())
    } else {
        Ok(())
    }
}
fn sample(
    region: Region,
    target: Option<&scroll::Target>,
    cancel: &AtomicBool,
) -> Result<Frame, String> {
    stable::sample(
        cancel,
        || {
            if let Some(target) = target {
                target.validate()?;
            }
            capture(region, cancel)
        },
        || wait(cancel, std::time::Duration::from_millis(60)),
    )
}
fn translated(region: Region, movement: (i32, i32), desktop: Region) -> Result<Region, String> {
    super::super::capture::validate_region(region.left, region.top, region.width, region.height)
        .map_err(|e| e.to_string())?;
    if movement.0.unsigned_abs() >= region.width as u32
        || movement.1.unsigned_abs() >= region.height as u32
    {
        return Err("Viewfinder movement must retain overlap".into());
    }
    let candidate = Region {
        left: region
            .left
            .checked_add(movement.0)
            .ok_or("Viewfinder coordinate overflow")?,
        top: region
            .top
            .checked_add(movement.1)
            .ok_or("Viewfinder coordinate overflow")?,
        ..region
    };
    super::super::capture::validate_region(
        candidate.left,
        candidate.top,
        candidate.width,
        candidate.height,
    )
    .map_err(|e| e.to_string())?;
    let right = i64::from(desktop.left) + i64::from(desktop.width);
    let bottom = i64::from(desktop.top) + i64::from(desktop.height);
    if desktop.width <= 0
        || desktop.height <= 0
        || candidate.left < desktop.left
        || candidate.top < desktop.top
        || i64::from(candidate.left) + i64::from(candidate.width) > right
        || i64::from(candidate.top) + i64::from(candidate.height) > bottom
    {
        return Err("Moved viewfinder exceeds virtual desktop".into());
    }
    Ok(candidate)
}
#[cfg(test)]
#[path = "long_capture_source_owner_tests.rs"]
mod tests;

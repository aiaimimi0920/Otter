//! Live 输入坐标转换；线程 DPI 上下文严格作用域恢复，不授予控制权限。
use super::{input_target::Destination, Target};
use windows::Win32::{
    Foundation::*,
    Graphics::{Dwm::*, Gdi::*},
    UI::{HiDpi::*, WindowsAndMessaging::*},
};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    InvalidPoint,
    InvalidBounds,
    Overflow,
    TargetChanged,
    Dpi,
    Transform,
}
type Result<T> = std::result::Result<T, Error>;
#[path = "live_input_dpi_bridge.rs"]
mod bridge;
/// 相对 WGC/DWM frame 的物理像素裁剪；仅窗口内区域，不是桌面区域控制授权。
#[derive(Clone, Copy, Debug)]
pub struct FrameRegion {
    pub left: u32,
    pub top: u32,
    pub width: u32,
    pub height: u32,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Mapped {
    pub root_client: [i32; 2],
    pub destination_client: [i32; 2],
    pub screen: [i32; 2],
}
pub(crate) struct Dpi(DPI_AWARENESS_CONTEXT);
impl Dpi {
    pub(crate) fn enter(window: HWND) -> Result<Self> {
        let context = unsafe { GetWindowDpiAwarenessContext(window) };
        if context.0.is_null() {
            return Err(Error::Dpi);
        }
        let old = unsafe { SetThreadDpiAwarenessContext(context) };
        if old.0.is_null() {
            return Err(Error::Dpi);
        }
        Ok(Self(old))
    }
}
impl Drop for Dpi {
    fn drop(&mut self) {
        unsafe {
            let _ = SetThreadDpiAwarenessContext(self.0);
        }
    }
}
/// 全窗口输入沿用主干客户区归一化规则；窗口内裁剪从物理 frame 转换。
/// destination 必须属于同一个原 root；结果仍需 message 层检查 signed 16-bit 范围。
pub fn map(
    root: &Target,
    destination: &Destination,
    normalized: [f64; 2],
    region: Option<FrameRegion>,
) -> Result<Mapped> {
    validate_point(normalized)?;
    if !root.same_source(destination.root()) {
        return Err(Error::TargetChanged);
    }
    root.validate().map_err(|_| Error::TargetChanged)?;
    destination.validate().map_err(|_| Error::TargetChanged)?;
    if unsafe { IsIconic(root.hwnd()) }.as_bool() {
        return Err(Error::InvalidBounds);
    }
    let _dpi = Dpi::enter(root.hwnd())?;
    let root_context = unsafe { GetWindowDpiAwarenessContext(root.hwnd()) };
    let destination_context = unsafe { GetWindowDpiAwarenessContext(destination.hwnd()) };
    let root_physical = unsafe { GetAwarenessFromDpiAwarenessContext(root_context) }
        == DPI_AWARENESS_PER_MONITOR_AWARE;
    let same_context =
        unsafe { AreDpiAwarenessContextsEqual(root_context, destination_context) }.as_bool();
    let mut point = if let Some(region) = region {
        let bounds = frame_bounds(root.hwnd())?;
        let physical = region_point(bounds, region, normalized)?;
        let mut point = POINT {
            x: physical[0],
            y: physical[1],
        };
        if (!root_physical
            && !unsafe { PhysicalToLogicalPointForPerMonitorDPI(Some(root.hwnd()), &mut point) }
                .as_bool())
            || !unsafe { ScreenToClient(root.hwnd(), &mut point) }.as_bool()
        {
            return Err(Error::Transform);
        }
        point
    } else {
        let mut bounds = RECT::default();
        unsafe { GetClientRect(root.hwnd(), &mut bounds) }.map_err(|_| Error::Transform)?;
        let point = rectangle_point(bounds, normalized)?;
        POINT {
            x: point[0],
            y: point[1],
        }
    };
    let root_client = [point.x, point.y];
    if !unsafe { ClientToScreen(root.hwnd(), &mut point) }.as_bool() {
        return Err(Error::Transform);
    }
    // 子控件可能属于不同 DPI awareness 的进程：通过物理屏幕坐标桥接，
    // 不能把 root 逻辑屏幕坐标直接当作 destination 的坐标。
    // 相同 awareness 直接映射；不要让隐藏窗口上的冗余 DPI API 调用拒绝合法的拖动坐标。
    if !same_context
        && !root_physical
        && !unsafe { LogicalToPhysicalPointForPerMonitorDPI(Some(root.hwnd()), &mut point) }
            .as_bool()
    {
        return Err(Error::Transform);
    }
    let _destination_dpi = Dpi::enter(destination.hwnd())?;
    let destination_physical = unsafe { GetAwarenessFromDpiAwarenessContext(destination_context) }
        == DPI_AWARENESS_PER_MONITOR_AWARE;
    if !same_context && !destination_physical {
        point = bridge::destination_point(root, destination, point)?;
    }
    let screen = [point.x, point.y];
    if !unsafe { ScreenToClient(destination.hwnd(), &mut point) }.as_bool() {
        return Err(Error::Transform);
    }
    root.validate().map_err(|_| Error::TargetChanged)?;
    destination.validate().map_err(|_| Error::TargetChanged)?;
    Ok(Mapped {
        root_client,
        screen,
        destination_client: [point.x, point.y],
    })
}
fn validate_point(point: [f64; 2]) -> Result<()> {
    if point
        .iter()
        .any(|value| !value.is_finite() || !(0.0..=1.0).contains(value))
    {
        return Err(Error::InvalidPoint);
    }
    Ok(())
}
fn rectangle_point(bounds: RECT, point: [f64; 2]) -> Result<[i32; 2]> {
    validate_point(point)?;
    let width = i64::from(bounds.right) - i64::from(bounds.left);
    let height = i64::from(bounds.bottom) - i64::from(bounds.top);
    if width <= 0 || height <= 0 {
        return Err(Error::InvalidBounds);
    }
    Ok([
        // 先舍入非负局部像素再加屏幕原点，避免负屏幕半像素因 round 方向不同偏移。
        (i64::from(bounds.left) + ((width - 1) as f64 * point[0]).round() as i64) as i32,
        (i64::from(bounds.top) + ((height - 1) as f64 * point[1]).round() as i64) as i32,
    ])
}
fn region_point(bounds: RECT, region: FrameRegion, point: [f64; 2]) -> Result<[i32; 2]> {
    let width = i64::from(bounds.right) - i64::from(bounds.left);
    let height = i64::from(bounds.bottom) - i64::from(bounds.top);
    if region.width == 0
        || region.height == 0
        || i64::from(region.left) + i64::from(region.width) > width
        || i64::from(region.top) + i64::from(region.height) > height
    {
        return Err(Error::InvalidBounds);
    }
    let left = i64::from(bounds.left) + i64::from(region.left);
    let top = i64::from(bounds.top) + i64::from(region.top);
    let convert = |value| i32::try_from(value).map_err(|_| Error::Overflow);
    rectangle_point(
        RECT {
            left: convert(left)?,
            top: convert(top)?,
            right: convert(left + i64::from(region.width))?,
            bottom: convert(top + i64::from(region.height))?,
        },
        point,
    )
}
pub(super) fn frame_bounds(window: HWND) -> Result<RECT> {
    let mut bounds = RECT::default();
    if unsafe {
        DwmGetWindowAttribute(
            window,
            DWMWA_EXTENDED_FRAME_BOUNDS,
            (&mut bounds as *mut RECT).cast(),
            std::mem::size_of::<RECT>() as u32,
        )
    }
    .is_err()
    {
        // GetWindowRect 会 DPI 虚拟化；失败回退必须临时使用 PMv2 获取物理边界。
        let old =
            unsafe { SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) };
        if old.0.is_null() {
            return Err(Error::Dpi);
        }
        let _dpi = Dpi(old);
        unsafe { GetWindowRect(window, &mut bounds) }.map_err(|_| Error::Transform)?;
    }
    Ok(bounds)
}
#[cfg(test)]
#[path = "live_input_dpi_runtime_tests.rs"]
mod dpi_runtime_tests;
#[cfg(test)]
#[path = "live_input_mixed_dpi_tests.rs"]
mod mixed_dpi_tests;
#[cfg(test)]
#[path = "live_input_coordinates_tests.rs"]
mod tests;

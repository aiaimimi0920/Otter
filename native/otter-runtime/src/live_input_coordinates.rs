//! Live input-coordinate conversion; strictly scoped restoration of thread DPI contexts, without granting control permissions.
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
    SourceTransition,
    Overflow,
    TargetChanged,
    Dpi,
    Transform,
}
type Result<T> = std::result::Result<T, Error>;
#[path = "live_input_dpi_bridge.rs"]
pub mod bridge;
/// Physical-pixel crop relative to the WGC/DWM frame; an in-window region only, not desktop-region control authorization.
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
pub struct Dpi(DPI_AWARENESS_CONTEXT);
impl Dpi {
    pub fn enter(window: HWND) -> Result<Self> {
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
/// The legacy full-window protocol retains client-area normalization; in-window crops convert from the physical frame.
/// destination must belong to the same original root; the message layer must still check the signed 16-bit range.
pub fn map(
    root: &Target,
    destination: &Destination,
    normalized: [f64; 2],
    region: Option<FrameRegion>,
) -> Result<Mapped> {
    map_in_space(root, destination, normalized, region, false)
}
/// Local ordinary-application images come from the complete WGC frame, including title bar/borders; do not stretch to the client area.
/// Change only coordinate space; do not turn nonclient areas into new messages or grant extra target permissions.
pub fn map_capture(
    root: &Target,
    destination: &Destination,
    normalized: [f64; 2],
    region: Option<FrameRegion>,
) -> Result<Mapped> {
    map_in_space(root, destination, normalized, region, true)
}
fn map_in_space(
    root: &Target,
    destination: &Destination,
    normalized: [f64; 2],
    region: Option<FrameRegion>,
    full_frame: bool,
) -> Result<Mapped> {
    validate_point(normalized)?;
    if !root.same_source(destination.root()) {
        return Err(Error::TargetChanged);
    }
    root.validate().map_err(|_| Error::TargetChanged)?;
    destination.validate().map_err(|_| Error::TargetChanged)?;
    if unsafe { IsIconic(root.hwnd()) }.as_bool() {
        return Err(Error::SourceTransition);
    }
    let _dpi = Dpi::enter(root.hwnd())?;
    let root_context = unsafe { GetWindowDpiAwarenessContext(root.hwnd()) };
    let destination_context = unsafe { GetWindowDpiAwarenessContext(destination.hwnd()) };
    let root_physical = unsafe { GetAwarenessFromDpiAwarenessContext(root_context) }
        == DPI_AWARENESS_PER_MONITOR_AWARE;
    let same_context =
        unsafe { AreDpiAwarenessContextsEqual(root_context, destination_context) }.as_bool();
    let mut point = if full_frame || region.is_some() {
        let bounds = frame_bounds(root.hwnd())?;
        let physical = match region {
            Some(region) => region_point(bounds, region, normalized)?,
            None => rectangle_point(bounds, normalized)?,
        };
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
    // Child controls may belong to processes with different DPI awareness: bridge through physical screen coordinates,
    // rather than treating root's logical screen coordinates as destination coordinates.
    // Map directly for matching awareness; redundant DPI API calls on hidden windows must not reject valid drag coordinates.
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
        // Round nonnegative local pixels before adding the screen origin to avoid negative-screen half-pixel shifts from different rounding directions.
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
        // GetWindowRect is DPI-virtualized; failure fallback must temporarily use PMv2 to obtain physical bounds.
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
#[path = "live_input_frame_mapping_tests.rs"]
mod frame_mapping_tests;
#[cfg(test)]
#[path = "live_input_mixed_dpi_tests.rs"]
mod mixed_dpi_tests;
#[cfg(test)]
#[path = "live_input_coordinates_tests.rs"]
mod tests;

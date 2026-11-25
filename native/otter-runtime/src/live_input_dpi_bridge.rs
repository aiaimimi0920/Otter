//! Out-of-control drag conversion for mixed hosting; do not truncate drag positions or guess cross-monitor scaling.
use super::*;

pub(super) fn destination_point(
    root: &Target,
    child: &Destination,
    physical: POINT,
) -> Result<POINT> {
    let mut converted = physical;
    if unsafe { PhysicalToLogicalPointForPerMonitorDPI(Some(child.hwnd()), &mut converted) }
        .as_bool()
    {
        return Ok(converted);
    }
    // Some windows accept only interior points. Use a validated interior point to obtain the system transform origin,
    // then extrapolate from the same-monitor PM parent/child DPI, rather than guessing a ratio from rounded window widths.
    let context = unsafe { GetWindowDpiAwarenessContext(root.hwnd()) };
    if unsafe { GetAwarenessFromDpiAwarenessContext(context) } != DPI_AWARENESS_PER_MONITOR_AWARE {
        return Err(Error::Transform);
    }
    let monitor = unsafe { MonitorFromWindow(root.hwnd(), MONITOR_DEFAULTTONULL) };
    if monitor.0.is_null()
        || monitor != unsafe { MonitorFromWindow(child.hwnd(), MONITOR_DEFAULTTONULL) }
    {
        return Err(Error::Transform);
    }
    let physical_dpi = unsafe { GetDpiForWindow(root.hwnd()) };
    let logical_dpi = unsafe { GetDpiForWindow(child.hwnd()) };
    if physical_dpi == 0 || logical_dpi == 0 {
        return Err(Error::Dpi);
    }
    let _dpi = Dpi::enter(root.hwnd())?;
    let mut bounds = RECT::default();
    unsafe { GetWindowRect(child.hwnd(), &mut bounds) }.map_err(|_| Error::Transform)?;
    if bounds.right <= bounds.left || bounds.bottom <= bounds.top {
        return Err(Error::InvalidBounds);
    }
    let anchor = POINT {
        x: ((i64::from(bounds.left) + i64::from(bounds.right)) / 2) as i32,
        y: ((i64::from(bounds.top) + i64::from(bounds.bottom)) / 2) as i32,
    };
    let mut logical = anchor;
    if !unsafe { PhysicalToLogicalPointForPerMonitorDPI(Some(child.hwnd()), &mut logical) }
        .as_bool()
    {
        return Err(Error::Transform);
    }
    Ok(POINT {
        x: extrapolate(physical.x, anchor.x, logical.x, physical_dpi, logical_dpi)?,
        y: extrapolate(physical.y, anchor.y, logical.y, physical_dpi, logical_dpi)?,
    })
}
fn extrapolate(value: i32, origin: i32, output: i32, from: u32, to: u32) -> Result<i32> {
    if from == 0 || to == 0 {
        return Err(Error::Dpi);
    }
    let numerator = (i64::from(value) - i64::from(origin))
        .checked_mul(i64::from(to))
        .ok_or(Error::Overflow)?;
    let magnitude = numerator.checked_abs().ok_or(Error::Overflow)?;
    let rounded = magnitude
        .checked_add(i64::from(from) / 2)
        .ok_or(Error::Overflow)?
        / i64::from(from);
    let result = i64::from(output)
        .checked_add(if numerator < 0 { -rounded } else { rounded })
        .ok_or(Error::Overflow)?;
    i32::try_from(result).map_err(|_| Error::Overflow)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn extrapolation_keeps_outside_coordinates_and_rejects_overflow() {
        assert_eq!(extrapolate(0, 150, 100, 144, 96), Ok(0));
        assert_eq!(extrapolate(-150, 150, 100, 144, 96), Ok(-100));
        assert_eq!(extrapolate(450, 150, 100, 144, 96), Ok(300));
        assert_eq!(extrapolate(0, 0, 0, 0, 96), Err(Error::Dpi));
        assert_eq!(
            extrapolate(i32::MAX, i32::MIN, 0, 1, u32::MAX),
            Err(Error::Overflow)
        );
    }
}

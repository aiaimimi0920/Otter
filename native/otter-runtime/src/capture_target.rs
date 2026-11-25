//! Bind physical desktop selections to single-monitor WGC regions; never truncate cross-monitor selections or deliver stale-coordinate results after layout changes.
use crate::{
    capture_color::{self, ScRgb},
    capture_readback::Region,
    capture_wgc,
};
use std::sync::atomic::{AtomicBool, Ordering};
use windows::Win32::{Foundation::RECT, Graphics::Gdi::*, UI::HiDpi::*};
#[derive(Debug)]
pub enum Error {
    Dimensions,
    NoMonitor,
    CrossMonitor,
    Changed,
    Cancelled,
    Api(windows::core::Error),
    Capture(capture_wgc::Error),
}
#[derive(Clone, Copy, Debug, PartialEq)]
struct Identity {
    monitor: HMONITOR,
    bounds: RECT,
    name: [u16; 32],
}
pub struct Target {
    screen: RECT,
    identity: Identity,
    region: Region,
}
impl Target {
    /// Query display layout only, without starting capture; screen must be a physical desktop rectangle with exclusive right/bottom edges.
    pub fn resolve(screen: RECT) -> Result<Self, Error> {
        dimensions(screen)?;
        let identity = locate(screen)?;
        let region = local(screen, identity.bounds)?;
        Ok(Self {
            screen,
            identity,
            region,
        })
    }
    pub fn monitor(&self) -> HMONITOR {
        self.identity.monitor
    }
    pub fn region(&self) -> Region {
        self.region
    }
    /// Revalidate physical bindings before continuous-capture delivery; failures discard candidates without changing old selections.
    pub fn validate(&self) -> Result<(), Error> {
        self.validate_with(|| locate(self.screen))
    }
    fn validate_with(&self, lookup: impl FnOnce() -> Result<Identity, Error>) -> Result<(), Error> {
        if lookup()? != self.identity {
            Err(Error::Changed)
        } else {
            Ok(())
        }
    }
    /// For explicit captures on background MTA threads; validate before/after capture and discard changed results rather than modify selections/retry.
    /// consume performs in-memory conversion only; file writes, clipboard publication, and UI updates must await successful return.
    pub fn capture<T>(
        &self,
        cancel: &AtomicBool,
        consume: impl FnOnce(&ScRgb<'_>) -> Result<T, capture_color::Error>,
    ) -> Result<T, Error> {
        self.run(
            cancel,
            || locate(self.screen),
            || {
                capture_wgc::monitor_region(self.identity.monitor, self.region, cancel, consume)
                    .map_err(Error::Capture)
            },
        )
    }
    pub fn capture_sdr(&self, cancel: &AtomicBool) -> Result<capture_color::SdrFrame, Error> {
        self.run(
            cancel,
            || locate(self.screen),
            || {
                capture_wgc::monitor_region_sdr(self.identity.monitor, self.region, cancel)
                    .map_err(Error::Capture)
            },
        )
    }
    fn run<T>(
        &self,
        cancel: &AtomicBool,
        mut lookup: impl FnMut() -> Result<Identity, Error>,
        capture: impl FnOnce() -> Result<T, Error>,
    ) -> Result<T, Error> {
        check(cancel)?;
        self.validate_with(&mut lookup)?;
        check(cancel)?;
        let result = capture()?;
        check(cancel)?;
        self.validate_with(&mut lookup)?;
        check(cancel)?;
        Ok(result)
    }
}
pub fn dimensions(rect: RECT) -> Result<(u32, u32), Error> {
    let width = i64::from(rect.right) - i64::from(rect.left);
    let height = i64::from(rect.bottom) - i64::from(rect.top);
    let width = u32::try_from(width).map_err(|_| Error::Dimensions)?;
    let height = u32::try_from(height).map_err(|_| Error::Dimensions)?;
    crate::viewport::checked_image_bytes(width, height).ok_or(Error::Dimensions)?;
    Ok((width, height))
}
fn local(screen: RECT, bounds: RECT) -> Result<Region, Error> {
    let (width, height) = dimensions(screen)?;
    dimensions(bounds)?;
    if screen.left < bounds.left
        || screen.top < bounds.top
        || screen.right > bounds.right
        || screen.bottom > bounds.bottom
    {
        return Err(Error::CrossMonitor);
    }
    Ok(Region {
        left: (i64::from(screen.left) - i64::from(bounds.left)) as u32,
        top: (i64::from(screen.top) - i64::from(bounds.top)) as u32,
        width,
        height,
    })
}
struct Dpi(DPI_AWARENESS_CONTEXT);
impl Drop for Dpi {
    fn drop(&mut self) {
        unsafe {
            let _ = SetThreadDpiAwarenessContext(self.0);
        }
    }
}
fn locate(screen: RECT) -> Result<Identity, Error> {
    unsafe {
        let previous = SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        if previous.0.is_null() {
            return Err(Error::Api(windows::core::Error::from_win32()));
        }
        let _dpi = Dpi(previous);
        let monitor = MonitorFromRect(&screen, MONITOR_DEFAULTTONULL);
        if monitor.0.is_null() {
            return Err(Error::NoMonitor);
        }
        let mut info = MONITORINFOEXW::default();
        info.monitorInfo.cbSize = std::mem::size_of::<MONITORINFOEXW>() as u32;
        GetMonitorInfoW(monitor, &mut info.monitorInfo)
            .ok()
            .map_err(Error::Api)?;
        Ok(Identity {
            monitor,
            bounds: info.monitorInfo.rcMonitor,
            name: info.szDevice,
        })
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
#[path = "capture_target_tests.rs"]
mod tests;

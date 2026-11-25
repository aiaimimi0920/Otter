//! Physical positioning of the initial Live client area; enter the target monitor before calculating nonclient bounds, without changing capture scope.
use super::*;
use windows::Win32::UI::HiDpi::{AdjustWindowRectExForDpi, GetDpiForWindow};

fn dimensions(rect: RECT) -> Result<D2D_SIZE_U> {
    let width = u32::try_from(i64::from(rect.right) - i64::from(rect.left))
        .map_err(|_| Error::new(E_INVALIDARG, "Invalid Live placement width"))?;
    let height = u32::try_from(i64::from(rect.bottom) - i64::from(rect.top))
        .map_err(|_| Error::new(E_INVALIDARG, "Invalid Live placement height"))?;
    let size = D2D_SIZE_U { width, height };
    super::super::super::validate_size(size)?;
    Ok(size)
}
fn outer(screen: RECT, inset: RECT) -> Result<RECT> {
    let add = |value: i32, delta: i32| {
        value
            .checked_add(delta)
            .ok_or_else(|| Error::new(E_INVALIDARG, "Live placement overflows desktop coordinates"))
    };
    Ok(RECT {
        left: add(screen.left, inset.left)?,
        top: add(screen.top, inset.top)?,
        right: add(screen.left, inset.right)?,
        bottom: add(screen.top, inset.bottom)?,
    })
}
impl LiveWindow {
    /// Callers require PMv2 context; remain hidden on return and start workers only after successful capture exclusion and positioning.
    pub fn new_at(consumer: Consumer<Texture>, screen: RECT) -> Result<Self> {
        Self::create_at_with(consumer, screen, |window, size| {
            window.place_content(screen, size)
        })
    }
    fn create_at_with(
        consumer: Consumer<Texture>,
        screen: RECT,
        place: impl FnOnce(&Self, D2D_SIZE_U) -> Result<()>,
    ) -> Result<Self> {
        let size = dimensions(screen)?;
        let window = Self::new(consumer, size.width, size.height)?;
        place(&window, size)?;
        Ok(window)
    }
    fn place_content(&self, screen: RECT, size: D2D_SIZE_U) -> Result<()> {
        let window = self;
        unsafe {
            // Do not steal focus or change Z order; the first move determines HWND DPI for the target monitor.
            SetWindowPos(
                window.hwnd,
                None,
                screen.left,
                screen.top,
                size.width as i32,
                size.height as i32,
                SWP_NOACTIVATE | SWP_NOZORDER,
            )?;
        }
        place_client(self.hwnd, screen)
    }
}
/// Adjust existing client areas using initialization's nonclient compensation; exclude title-bar dimensions from image scaling ratios.
pub(super) fn place_client(hwnd: HWND, screen: RECT) -> Result<()> {
    place_client_while(hwnd, screen, || true)
}
/// Revalidate evidence before/after each reentrant positioning call; old drags must not override newer operations or cancellation results.
pub(super) fn place_client_while(
    hwnd: HWND,
    screen: RECT,
    current: impl Fn() -> bool,
) -> Result<()> {
    let size = dimensions(screen)?;
    unsafe {
        // Nonclient expansion may cross mixed-DPI monitor seams; apply bounded correction, never a repositioning loop.
        for _ in 0..3 {
            if !current() {
                return Err(Error::new(E_ABORT, "Live placement superseded"));
            }
            let dpi = GetDpiForWindow(hwnd);
            if dpi == 0 {
                return Err(Error::new(E_FAIL, "Live placement DPI unavailable"));
            }
            let mut inset = RECT {
                left: 0,
                top: 0,
                right: size.width as i32,
                bottom: size.height as i32,
            };
            AdjustWindowRectExForDpi(
                &mut inset,
                WINDOW_STYLE(GetWindowLongPtrW(hwnd, GWL_STYLE) as u32),
                false,
                WINDOW_EX_STYLE(GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32),
                dpi,
            )?;
            let frame = outer(screen, inset)?;
            SetWindowPos(
                hwnd,
                None,
                frame.left,
                frame.top,
                frame.right - frame.left,
                frame.bottom - frame.top,
                // Precise initial positioning is not user edge-dragging; prevent default minimum tracking width from enlarging valid narrow selections.
                // Still receive WM_WINDOWPOSCHANGED/WM_SIZE to preserve normal presentation-size invalidation.
                SWP_NOACTIVATE | SWP_NOZORDER | SWP_NOSENDCHANGING,
            )?;
            if !current() {
                return Err(Error::new(E_ABORT, "Live placement superseded"));
            }
            // Verify exact client bounds, not just outer bounds, rejecting silent offsets/scaling from system constraints.
            let mut origin = POINT::default();
            let mut client = RECT::default();
            GetClientRect(hwnd, &mut client)?;
            if !ClientToScreen(hwnd, &mut origin).as_bool() {
                return Err(Error::from_win32());
            }
            if origin.x == screen.left
                && origin.y == screen.top
                && client.right == size.width as i32
                && client.bottom == size.height as i32
            {
                return Ok(());
            }
        }
    }
    Err(Error::new(
        E_FAIL,
        "Live client placement does not match requested physical rectangle",
    ))
}

#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-live")))]
#[path = "composition_live_placement_tests.rs"]
mod tests;

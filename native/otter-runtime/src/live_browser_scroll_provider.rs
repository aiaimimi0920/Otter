//! Find the deepest scroll container under the pointer only within the bound source's UIA subtree; no desktop hit testing or text reads.
use super::*;
use windows::Win32::UI::HiDpi::*;
use windows::{
    core::*,
    Win32::{Foundation::*, System::Com::*, UI::Accessibility::*},
};
struct PhysicalDpi(DPI_AWARENESS_CONTEXT);
impl Drop for PhysicalDpi {
    fn drop(&mut self) {
        unsafe {
            let _ = SetThreadDpiAwarenessContext(self.0);
        }
    }
}
struct Apartment;
impl Drop for Apartment {
    fn drop(&mut self) {
        unsafe {
            CoUninitialize();
        }
    }
}
pub(super) struct Provider {
    client: IUIAutomation,
    cache: IUIAutomationCacheRequest,
    target: Target,
    _apartment: Apartment,
    _dpi: PhysicalDpi,
}
pub(super) struct Prepared {
    element: IUIAutomationElement,
    pattern: IUIAutomationScrollPattern,
    point: [i32; 2],
    ticks: i32,
    horizontal: bool,
}
impl Provider {
    pub(super) fn new(target: Target) -> Result<Self> {
        unsafe {
            // The UIA Win32 provider may still virtualize rectangles by client-thread DPI; protocol points always use physical pixels.
            let previous = SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
            if previous.0.is_null() {
                return Err(Error::from_win32());
            }
            let dpi = PhysicalDpi(previous);
            CoInitializeEx(None, COINIT_MULTITHREADED).ok()?;
            let apartment = Apartment;
            let client: IUIAutomation2 =
                CoCreateInstance(&CUIAutomation8, None, CLSCTX_INPROC_SERVER)?;
            // Live controls background A; UIA patterns' default focus behavior must not steal input focus from B.
            client.SetAutoSetFocus(false)?;
            client.SetConnectionTimeout(200)?;
            client.SetTransactionTimeout(200)?;
            let client: IUIAutomation = client.cast()?;
            // Single-node caching merges multiple cross-process property RPCs into the navigation request; do not retain old page nodes across events.
            let cache = client.CreateCacheRequest()?;
            for property in [
                UIA_ProcessIdPropertyId,
                UIA_IsPasswordPropertyId,
                UIA_BoundingRectanglePropertyId,
                UIA_ControlTypePropertyId,
                UIA_ScrollHorizontallyScrollablePropertyId,
                UIA_ScrollVerticallyScrollablePropertyId,
            ] {
                cache.AddProperty(property)?;
            }
            cache.AddPattern(UIA_ScrollPatternId)?;
            // The initial connection starts the browser accessibility tree; rebind on later scrolls rather than caching old page nodes.
            let root = client.ElementFromHandleBuildCache(target.hwnd(), &cache)?;
            warm_document(&client, &cache, &root, target.process_id())?;
            Ok(Self {
                client,
                cache,
                target,
                _apartment: apartment,
                _dpi: dpi,
            })
        }
    }
    pub(super) fn prepare(
        &self,
        point: [i32; 2],
        ticks: i32,
        horizontal: bool,
    ) -> Result<Option<Prepared>> {
        self.target.validate()?;
        crate::live_window_target::input_security::preflight(&self.target)
            .map_err(|_| Error::from(E_ACCESSDENIED))?;
        let deadline = Instant::now() + Duration::from_millis(450);
        // A may be scrolled independently by users or page scripts. Refresh first, then query the tree and read the current position,
        // or UIA relative Scroll will derive an absolute target from stale AX coordinates and pull the page back to its old position.
        super::refresh::position(&self.target, point)?;
        unsafe {
            let root = self
                .client
                .ElementFromHandleBuildCache(self.target.hwnd(), &self.cache)?;
            if root.CachedProcessId()? != self.target.process_id() as i32 {
                return Err(Error::from(E_ACCESSDENIED));
            }
            let walker = visible_walker(&self.client)?;
            let mut stack = vec![(root, 0usize)];
            let mut selected = None;
            let mut deepest = 0;
            let mut visited = 0;
            while let Some((element, depth)) = stack.pop() {
                if Instant::now() >= deadline {
                    return Err(Error::from(HRESULT(0x800705B4u32 as i32)));
                }
                visited += 1;
                if visited > 256 || depth > 32 {
                    return Err(Error::from(E_BOUNDS));
                }
                if element.CachedProcessId()? != self.target.process_id() as i32
                    || element.CachedIsPassword()?.as_bool()
                {
                    continue;
                }
                let bounds = element.CachedBoundingRectangle()?;
                // Chromium virtual grouping nodes may have no rectangle while their descendants still contain visible scroll containers.
                // Prune only nonempty branches known not to contain the pointer; rectangle-less bridge nodes must not be treated as leaves.
                if bounds.right > bounds.left
                    && bounds.bottom > bounds.top
                    && !contains(bounds, point)
                {
                    continue;
                }
                if let Some(pattern) = optional(
                    element.GetCachedPatternAs::<IUIAutomationScrollPattern>(UIA_ScrollPatternId),
                )? {
                    let available = if horizontal {
                        pattern.CachedHorizontallyScrollable()?
                    } else {
                        pattern.CachedVerticallyScrollable()?
                    };
                    if contains(bounds, point) && available.as_bool() && depth >= deepest {
                        deepest = depth;
                        selected = Some((element.clone(), pattern));
                    }
                }
                let mut child =
                    optional(walker.GetFirstChildElementBuildCache(&element, &self.cache))?;
                while let Some(next) = child {
                    if Instant::now() >= deadline {
                        return Err(Error::from(HRESULT(UIA_E_TIMEOUT as i32)));
                    }
                    if visited + stack.len() >= 256 {
                        return Err(Error::from(E_BOUNDS));
                    }
                    child = optional(walker.GetNextSiblingElementBuildCache(&next, &self.cache))?;
                    stack.push((next, depth + 1));
                }
            }
            Ok(selected.map(|(element, pattern)| Prepared {
                element,
                pattern,
                point,
                ticks,
                horizontal,
            }))
        }
    }
    pub(super) fn scroll(
        &self,
        prepared: Prepared,
        deadline: Instant,
        attempted: &mut bool,
    ) -> Result<bool> {
        crate::live_window_target::input_security::preflight(&self.target)
            .map_err(|_| Error::from(E_ACCESSDENIED))?;
        let Prepared {
            element,
            pattern,
            point,
            ticks,
            horizontal,
        } = prepared;
        unsafe {
            let increment = if horizontal { ticks > 0 } else { ticks < 0 };
            let amount = if increment {
                ScrollAmount_SmallIncrement
            } else {
                ScrollAmount_SmallDecrement
            };
            for _ in 0..ticks.unsigned_abs() {
                if Instant::now() >= deadline {
                    return Err(Error::from(HRESULT(0x800705B4u32 as i32)));
                }
                self.target.validate()?;
                if element.CurrentProcessId()? != self.target.process_id() as i32
                    || !contains(element.CurrentBoundingRectangle()?, point)
                {
                    return Err(Error::from(E_ABORT));
                }
                let percent = || {
                    if horizontal {
                        pattern.CurrentHorizontalScrollPercent()
                    } else {
                        pattern.CurrentVerticalScrollPercent()
                    }
                };
                let before = percent()?;
                // Property RPCs may also exhaust the budget; the final check must immediately precede the action, which must not start after expiry.
                if Instant::now() >= deadline {
                    return Err(Error::from(HRESULT(0x800705B4u32 as i32)));
                }
                // One provider small step per notch; partial completion/timeouts have unknown outcomes, so do not fall back and resend the original wheel input.
                *attempted = true;
                pattern.Scroll(
                    if horizontal {
                        amount
                    } else {
                        ScrollAmount_NoAmount
                    },
                    if horizontal {
                        ScrollAmount_NoAmount
                    } else {
                        amount
                    },
                )?;
                // Successful Scroll does not mean the AX cache is updated; read-only hit requests encourage timely source feedback without replaying the action.
                if Instant::now() >= deadline {
                    return Err(Error::from(HRESULT(0x800705B4u32 as i32)));
                }
                super::refresh::position(&self.target, point)?;
                super::settle::observe(before, increment, deadline, percent)?;
            }
            Ok(true)
        }
    }
}
// Chromium's first UIA connection loads the page tree asynchronously. Wait for Document only within the startup budget,
// without executing or retrying scroll actions; browser windows without a page remain usable after expiry.
fn warm_document(
    client: &IUIAutomation,
    cache: &IUIAutomationCacheRequest,
    root: &IUIAutomationElement,
    pid: u32,
) -> Result<()> {
    let deadline = Instant::now() + Duration::from_millis(800);
    unsafe {
        let walker = visible_walker(client)?;
        loop {
            let mut stack = vec![root.clone()];
            let mut visited = 0;
            while let Some(element) = stack.pop() {
                if Instant::now() >= deadline {
                    return Ok(());
                }
                visited += 1;
                if visited > 256 {
                    return Err(Error::from(E_BOUNDS));
                }
                if element.CachedProcessId()? != pid as i32 || element.CachedIsPassword()?.as_bool()
                {
                    continue;
                }
                if element.CachedControlType()? == UIA_DocumentControlTypeId {
                    return Ok(());
                }
                let mut child = optional(walker.GetFirstChildElementBuildCache(&element, cache))?;
                while let Some(next) = child {
                    if Instant::now() >= deadline {
                        return Ok(());
                    }
                    if visited + stack.len() >= 256 {
                        return Err(Error::from(E_BOUNDS));
                    }
                    child = optional(walker.GetNextSiblingElementBuildCache(&next, cache))?;
                    stack.push(next);
                }
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    }
}
fn contains(r: RECT, p: [i32; 2]) -> bool {
    p[0] >= r.left && p[0] < r.right && p[1] >= r.top && p[1] < r.bottom
}
// Ordinary overflow containers may be outside ControlView while still providing ScrollPattern.
// Retain them to hit the deepest container; still exclude offscreen nodes without expanding to the entire RawView.
unsafe fn visible_walker(client: &IUIAutomation) -> Result<IUIAutomationTreeWalker> {
    let visible = client.CreatePropertyCondition(UIA_IsOffscreenPropertyId, &false.into())?;
    let scrollable =
        client.CreatePropertyCondition(UIA_IsScrollPatternAvailablePropertyId, &true.into())?;
    let candidates = client.CreateOrCondition(&client.ControlViewCondition()?, &scrollable)?;
    let condition = client.CreateAndCondition(&candidates, &visible)?;
    client.CreateTreeWalker(&condition)
}
fn optional<T>(result: Result<T>) -> Result<Option<T>> {
    match result {
        Ok(value) => Ok(Some(value)),
        Err(error)
            if error.code() == S_OK || error.code() == HRESULT(UIA_E_NOTSUPPORTED as i32) =>
        {
            Ok(None)
        }
        Err(error) => Err(error),
    }
}

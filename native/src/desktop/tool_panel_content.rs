//! Hidden pins defer only BUTTON creation; parent panel/font/capture-protection owners remain, with complete projection before first display.
use super::*;
const KEY: windows::core::PCWSTR = w!("Otter.ToolPanel.Content");
// A nonnull marker solely for Win32 property comparison, not a resource handle and never dereferenced.
const BUILDING: HANDLE = HANDLE(std::ptr::without_provenance_mut(1));
const READY: HANDLE = HANDLE(std::ptr::without_provenance_mut(2));

pub(super) unsafe fn ready(panel: HWND) -> bool {
    if neuro_mini_ui::is_toolbar(panel) {
        neuro_mini_ui::ready(panel)
    } else {
        GetPropW(panel, KEY) == READY
    }
}
struct Pending(HWND, bool);
impl Drop for Pending {
    fn drop(&mut self) {
        if !self.1 {
            unsafe {
                let _ = RemovePropW(self.0, KEY);
            }
        }
    }
}
pub(in crate::desktop) unsafe fn prepare(panel: HWND, state: &State) -> Result<()> {
    if neuro_mini_ui::is_toolbar(panel) {
        neuro_mini_ui::prepare(panel)?;
        strip::refresh(state);
        return Ok(());
    }
    prepare_with(panel, state, || {
        children::create(panel, HINSTANCE(GetModuleHandleW(None)?.0))
    })
}
pub(super) unsafe fn prepare_with(
    panel: HWND,
    state: &State,
    create: impl FnOnce() -> Result<Vec<Window>>,
) -> Result<()> {
    if ready(panel) {
        return Ok(());
    }
    if !GetPropW(panel, KEY).is_invalid() {
        return Err(windows::core::Error::new(
            E_UNEXPECTED,
            "Tool panel content creation reentered",
        ));
    }
    SetPropW(panel, KEY, Some(BUILDING))?;
    let mut pending = Pending(panel, false);
    // Synchronous creation callbacks may reenter; do not refresh caches while BUILDING, and release created children before clearing the marker on failure.
    let mut controls = crate::desktop::smoke_timing::run("tool-panel-controls", create)?;
    SetPropW(panel, KEY, Some(READY))?;
    for control in &mut controls {
        control.0 = HWND::default(); // Transfer to the parent HWND after all succeed; no fallible preparation follows.
    }
    pending.1 = true;
    state.tool_panel_enabled.set(u64::MAX);
    state.tool_panel_tool.set(None);
    state.tool_panel_style.set(None);
    crate::desktop::smoke_timing::run("tool-panel-project", || {
        refresh(HWND(GetWindowLongPtrW(panel, GWLP_USERDATA) as *mut _));
        Ok::<_, windows::core::Error>(())
    })?;
    Ok(())
}
pub(super) unsafe fn before_show(panel: HWND, state: &State, position: &mut WINDOWPOS) {
    if position.flags.contains(SWP_SHOWWINDOW) && !position.flags.contains(SWP_HIDEWINDOW) {
        if let Err(error) = prepare(panel, state) {
            // Failure remains hidden after temporary controls roll back; retry on next display rather than publish empty/partial panels.
            position.flags &= !SWP_SHOWWINDOW;
            position.flags |= SWP_HIDEWINDOW;
            eprintln!("native-tool-panel-content: {error}");
        }
    }
}

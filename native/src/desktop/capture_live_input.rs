//! 显式 Live 选区请求；复用冻结选区，不替换文档，消息回调后由 Application 启动会话。
use super::*;
pub(in crate::desktop) const COMMAND: usize = 0x106b;
pub(in crate::desktop) const WINDOW: usize = 0x106c;
#[path = "live_window_picker.rs"]
mod picker;
#[path = "capture_live_selection.rs"]
mod selection;
pub(in crate::desktop) enum Request {
    Region(RECT),
    Window(crate::live_window_target::Target),
    WindowRegion(
        crate::live_window_target::Target,
        crate::live_window_target::region::Crop,
    ),
}
pub(in crate::desktop) fn available(state: &State) -> bool {
    exclusive_available(state)
        && state
            .live_request
            .try_borrow()
            .is_ok_and(|request| request.is_none())
        && super::super::tool_panel_availability::Availability::capture(state)
            .is_some_and(|value| value.allows(NEW_PIN))
}
pub(in crate::desktop) unsafe fn start(hwnd: HWND, state: &State) {
    start_with(hwnd, state, false);
}
pub(in crate::desktop) unsafe fn start_window(hwnd: HWND, state: &State) {
    start_with(hwnd, state, true);
}
unsafe fn start_with(hwnd: HWND, state: &State, window: bool) {
    if !available(state) {
        return;
    }
    let Some(_capture) = super::super::file_dialog::Guard::begin(&state.capture_active) else {
        return;
    };
    let Some(_dialog) = super::super::file_dialog::Guard::begin(&state.file_dialog) else {
        return;
    };
    let result = (|| -> Result<()> {
        state.canvas.borrow_mut().cancel_gesture();
        if GetCapture() == hwnd {
            ReleaseCapture()?;
        }
        if window {
            if let Some(target) = picker::select(hwnd)? {
                if IsWindow(Some(hwnd)).as_bool() && !state.exit_requested.get() {
                    *state.live_request.borrow_mut() = Some(Request::Window(target));
                }
            }
            return Ok(());
        }
        let selected = {
            let _hidden = Hidden::for_state(hwnd, state);
            DwmFlush()?;
            selection::select()?
        };
        let Some(request) = selected else {
            return Ok(());
        };
        if IsWindow(Some(hwnd)).as_bool() && !state.exit_requested.get() {
            *state.live_request.borrow_mut() = Some(request);
        }
        Ok(())
    })();
    if let Err(error) = result {
        if IsWindow(Some(hwnd)).as_bool() {
            MessageBoxW(
                Some(hwnd),
                &HSTRING::from(format!("无法开始 Live。\n{error}")),
                w!("Otter"),
                MB_OK | MB_ICONWARNING,
            );
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn selection_bounds_and_pending_or_modal_requests_are_rejected() {
        let rect = RECT {
            left: -1920,
            top: -100,
            right: -1820,
            bottom: -50,
        };
        let state = State::new(None, true).unwrap();
        assert!(available(&state));
        *state.live_request.borrow_mut() = Some(Request::Region(rect));
        assert!(!available(&state));
        state.live_request.borrow_mut().take();
        state.file_dialog.set(true);
        assert!(!available(&state));
    }
}

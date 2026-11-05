//! 显式区域截图入口；截图前隐藏应用宿主，所有返回路径恢复窗口，复用后台加载提交。
use super::{image::Image, State};
use windows::{
    core::{w, Error, Result, HSTRING},
    Win32::{
        Foundation::*,
        Graphics::Dwm::DwmFlush,
        UI::{Input::KeyboardAndMouse::*, WindowsAndMessaging::*},
    },
};
pub(super) const COMMAND: usize = 0x1044;
pub(super) const FULL_SCREEN: usize = 0x1045;
pub(super) const NEW_PIN: usize = 0x104d;
pub(super) const NEW_FULL_PIN: usize = 0x104f;
pub(super) const LONG_VERTICAL: usize = 0x1066;
pub(super) const LONG_HORIZONTAL: usize = 0x1067;
pub(super) const LONG_RECORDING: usize = 0x10BE;
#[path = "capture_input_dynamic.rs"]
mod dynamic;
#[path = "long_capture_finish_route.rs"]
pub(super) mod finish_route;
#[path = "capture_live_input.rs"]
pub(super) mod live;
#[path = "long_capture_dialog.rs"]
mod long_capture_dialog;
#[path = "capture_route.rs"]
mod route;
use dynamic::Delivery;
#[path = "capture_info.rs"]
pub(super) mod info;
// 常用截图入口保留当前编辑；替换只保留在显式命名的菜单命令中。
pub(super) const DEFAULT: usize = NEW_PIN;
pub(super) unsafe fn append_pin(menu: HMENU) -> Result<()> {
    AppendMenuW(
        menu,
        MF_STRING,
        LONG_RECORDING,
        w!("长截图录制（自行滚动）"),
    )?;
    AppendMenuW(menu, MF_STRING, LONG_VERTICAL, w!("垂直长截图"))?;
    AppendMenuW(menu, MF_STRING, LONG_HORIZONTAL, w!("水平长截图"))?;
    AppendMenuW(menu, MF_STRING, NEW_PIN, w!("区域截图为新钉图"))?;
    AppendMenuW(
        menu,
        MF_STRING,
        NEW_FULL_PIN,
        w!("全屏截图为新钉图（所有显示器）"),
    )
}
pub(super) fn exclusive_available(state: &State) -> bool {
    !state.capture_active.get() && !state.file_dialog.get() && !state.modals.blocked()
}
#[path = "capture_input_smoke.rs"]
mod smoke;
pub(super) use smoke::run as smoke;
pub(super) use smoke::shared as smoke_shared;
struct Hidden(Vec<(HWND, bool, isize)>);
impl Hidden {
    unsafe fn for_state(hwnd: HWND, state: &State) -> Self {
        // 快照后释放共享列表借用；ShowWindow/截图选区消息可能重入宿主。
        let mut windows = state.capture_windows.borrow().clone();
        for own in [hwnd, state.tool_panel.get(), state.parameters_ui.window()] {
            if !windows.contains(&own) {
                windows.push(own);
            }
        }
        Self::new(&windows)
    }
    unsafe fn new(windows: &[HWND]) -> Self {
        let saved = windows
            .iter()
            .filter(|window| IsWindow(Some(**window)).as_bool())
            .map(|window| {
                (
                    *window,
                    IsWindowVisible(*window).as_bool(),
                    GetWindowLongPtrW(*window, GWLP_USERDATA),
                )
            })
            .collect::<Vec<_>>();
        let guard = Self(saved);
        for (window, visible, _) in &guard.0 {
            if *visible {
                let _ = ShowWindow(*window, SW_HIDE);
            }
        }
        guard
    }
}
impl Drop for Hidden {
    fn drop(&mut self) {
        unsafe {
            for (window, visible, identity) in &self.0 {
                if *visible
                    && IsWindow(Some(*window)).as_bool()
                    && GetWindowLongPtrW(*window, GWLP_USERDATA) == *identity
                {
                    let _ = ShowWindow(*window, SW_SHOWNOACTIVATE);
                }
            }
        }
    }
}
pub(super) unsafe fn start(hwnd: HWND, state: &State, command: usize) {
    let Some(route) = route::Route::resolve(command) else {
        return;
    };
    if !exclusive_available(state) {
        return;
    }
    if !super::tool_panel_availability::Availability::capture(state)
        .is_some_and(|value| value.allows(command))
    {
        return;
    }
    let Some(_capture) = super::file_dialog::Guard::begin(&state.capture_active) else {
        return;
    };
    let Some(_dialog) = super::file_dialog::Guard::begin(&state.file_dialog) else {
        return;
    };
    if !route.new_pin && !super::open_image::confirm_replace(hwnd, state) {
        return;
    }
    let result = run_delivery(hwnd, state, route.new_pin, || {
        // 等待隐藏窗口交付给 DWM，避免当前面板进入冻结帧。
        DwmFlush()?;
        let acquired = route.acquire_with(
            super::capture_windows::snapshot,
            super::capture::virtual_desktop,
            super::capture_region::select_positioned,
        )?;
        if command == LONG_RECORDING {
            match acquired {
                Some((image, Some(origin))) => {
                    long_capture_dialog::show_recording(hwnd, image, origin, &state.capture_active)
                        .map(|result| result.map(Delivery::Recording))
                }
                _ => Ok(None),
            }
        } else if matches!(command, LONG_VERTICAL | LONG_HORIZONTAL) {
            if let Some((image, Some(origin))) = acquired {
                let source_size = (image.width, image.height);
                let axis = if command == LONG_VERTICAL {
                    crate::long_capture::Axis::Vertical
                } else {
                    crate::long_capture::Axis::Horizontal
                };
                return long_capture_dialog::show(hwnd, image, origin, axis, &state.capture_active)
                    .map(|result| {
                        result.map(|image| Delivery::Long(image, origin, axis, source_size))
                    });
            }
            Ok(None)
        } else if matches!(command, COMMAND | NEW_PIN) {
            match acquired {
                Some((image, Some(origin))) => dynamic::prepare(hwnd, state, image, origin),
                _ => Ok(None),
            }
        } else {
            Ok(acquired.map(|(image, origin)| Delivery::Image(image, origin)))
        }
    });
    if let Err(error) = result {
        if IsWindow(Some(hwnd)).as_bool() {
            MessageBoxW(
                Some(hwnd),
                &HSTRING::from(format!("无法完成截图，原文档保留。\n{error}")),
                w!("Otter"),
                MB_OK | MB_ICONWARNING,
            );
        }
    }
}
unsafe fn run_with(
    hwnd: HWND,
    state: &State,
    capture: impl FnOnce() -> Result<Option<Image>>,
) -> Result<()> {
    run_destination(hwnd, state, false, capture)
}
pub(super) unsafe fn run_destination(
    hwnd: HWND,
    state: &State,
    new_pin: bool,
    capture: impl FnOnce() -> Result<Option<Image>>,
) -> Result<()> {
    run_positioned(hwnd, state, new_pin, || {
        capture().map(|result| result.map(|image| (image, None)))
    })
}
pub(super) unsafe fn run_positioned(
    hwnd: HWND,
    state: &State,
    new_pin: bool,
    capture: impl FnOnce() -> Result<Option<(Image, Option<POINT>)>>,
) -> Result<()> {
    run_delivery(hwnd, state, new_pin, || {
        capture().map(|result| result.map(|(image, origin)| Delivery::Image(image, origin)))
    })
}
unsafe fn run_delivery(
    hwnd: HWND,
    state: &State,
    new_pin: bool,
    capture: impl FnOnce() -> Result<Option<Delivery>>,
) -> Result<()> {
    state.canvas.borrow_mut().cancel_gesture();
    if GetCapture() == hwnd {
        ReleaseCapture()?;
    }
    let stamp = state.canvas.borrow().editor.document_stamp();
    let image = {
        let _hidden = Hidden::for_state(hwnd, state);
        capture()?
    };
    let Some(delivery) = image else {
        return Ok(());
    };
    if !IsWindow(Some(hwnd)).as_bool() {
        return Ok(());
    }
    if !stamp.matches(&state.canvas.borrow().editor) {
        return Err(Error::new(E_ABORT, "Document changed during capture"));
    }
    match delivery {
        Delivery::Recording(output) => {
            let (origin, context) = dynamic::recording_context(&output)?;
            super::open_job::start_long_context(hwnd, state, output.image, origin, context)
        }
        Delivery::Long(image, origin, axis, source_size) => {
            super::open_job::start_long(hwnd, state, image, origin, axis, source_size)
        }
        Delivery::Image(image, origin) => {
            if new_pin {
                super::open_job::start_pin(hwnd, state, image, origin)
            } else {
                super::open_job::start_image(hwnd, state, image, origin)
            }
        }
        Delivery::Prepared(prepared, origin) => {
            let captured = prepared
                .commit(&std::sync::atomic::AtomicBool::new(false), || {
                    IsWindow(Some(hwnd)).as_bool() && stamp.matches(&state.canvas.borrow().editor)
                })
                .map_err(|error| Error::new(E_FAIL, error.to_string()))?;
            super::open_job::start_destination(
                hwnd,
                state,
                super::open_job::Input::Captured(captured, Some(origin)),
                new_pin,
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn hidden_window_guard_preserves_hidden_state_and_tolerates_destroyed_windows() {
        unsafe {
            let window = crate::desktop::Window(
                CreateWindowExW(
                    WINDOW_EX_STYLE::default(),
                    w!("STATIC"),
                    w!("capture guard fixture"),
                    WS_POPUP,
                    0,
                    0,
                    1,
                    1,
                    None,
                    None,
                    None,
                    None,
                )
                .unwrap(),
            );
            let guard = Hidden::new(&[window.0, HWND::default()]);
            assert_eq!(guard.0.len(), 1);
            assert!(!IsWindowVisible(window.0).as_bool());
            drop(guard);
            assert!(!IsWindowVisible(window.0).as_bool());
            let guard = Hidden::new(&[window.0]);
            DestroyWindow(window.0).unwrap();
            drop(guard);
        }
    }
}

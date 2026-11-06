//! HWND 几何提交前准备滚轮帧，提交后仍沿默认尺寸通知适配视口及共享 chrome。
use super::State;
use windows::Win32::{Foundation::*, UI::WindowsAndMessaging::*};

pub(super) unsafe fn before_dispatch(
    hwnd: HWND,
    state: &State,
    msg: u32,
    wp: WPARAM,
    lp: LPARAM,
) -> Option<LRESULT> {
    crate::desktop::pin_window::preview::click_dispatch(hwnd, state, msg, wp, lp)
        .or_else(|| crate::desktop::pin_window::animation::dispatch(hwnd, state, msg, wp))
        .or_else(|| crate::desktop::session_end::dispatch(hwnd, state, msg, wp, lp))
}

pub(super) unsafe fn dispatch(
    hwnd: HWND,
    state: &State,
    msg: u32,
    wp: WPARAM,
    lp: LPARAM,
) -> Option<LRESULT> {
    if !matches!(
        msg,
        WM_NCCALCSIZE | WM_WINDOWPOSCHANGING | WM_WINDOWPOSCHANGED
    ) {
        return None;
    }
    let result = DefWindowProcW(hwnd, msg, wp, lp);
    if state.lifetime.owns(hwnd) {
        if msg == WM_WINDOWPOSCHANGING && lp.0 != 0 && state.pin_resize.rect.get().is_none() {
            let pos = &*(lp.0 as *const WINDOWPOS);
            let mut old = RECT::default();
            if !pos.flags.contains(SWP_NOMOVE | SWP_NOSIZE) && GetWindowRect(hwnd, &mut old).is_ok()
            {
                let (x, y) = if pos.flags.contains(SWP_NOMOVE) {
                    (old.left, old.top)
                } else {
                    (pos.x, pos.y)
                };
                // 普通移动也在宿主位置生效前准备目标外框，不在移动旧图后追画白框。
                let size = if pos.flags.contains(SWP_NOSIZE) {
                    (old.right - old.left, old.bottom - old.top)
                } else {
                    (pos.cx, pos.cy)
                };
                if size.0 > 0 && size.1 > 0 {
                    crate::desktop::pin_window::movement::position(
                        hwnd,
                        state,
                        crate::pin_geometry::PinRect {
                            x,
                            y,
                            width: size.0 as u32,
                            height: size.1 as u32,
                        },
                    );
                    crate::desktop::application::chrome::zoom_frame(
                        &state.chrome,
                        hwnd,
                        crate::pin_geometry::PinRect {
                            x,
                            y,
                            width: size.0 as u32,
                            height: size.1 as u32,
                        },
                    );
                }
            }
        } else if msg == WM_NCCALCSIZE {
            crate::desktop::pin_window::presentation::before_geometry(hwnd, state, wp, lp);
        } else if msg == WM_WINDOWPOSCHANGED && state.pin_resize.rect.get().is_none() {
            // 默认过程先派发 WM_SIZE 并适配视口，透明叠层才可同步触发宿主绘制。
            crate::desktop::application::chrome::geometry(&state.chrome, hwnd, msg);
        }
    }
    Some(result)
}

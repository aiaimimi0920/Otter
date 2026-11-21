//! 当前帧编辑复用普通钉图；黄色角返回保留中的 Live，会话不随工具栏显隐结束。
use super::*;
use windows::Win32::{Graphics::Gdi::*, UI::WindowsAndMessaging::*};
pub(super) unsafe fn dispatch(hwnd: HWND, state: &State, msg: u32, lp: LPARAM) -> Option<LRESULT> {
    if !state.live_pin.get() {
        return None;
    }
    if state.group_locked.get()
        || state.group_hidden.get()
        || state.session_end.paused()
        || super::group_lock::editing_busy(state)
        || !super::capture_input::exclusive_available(state)
    {
        return None;
    }
    if msg == WM_NCHITTEST {
        let mut point = POINT {
            x: lp.0 as i16 as i32,
            y: (lp.0 >> 16) as i16 as i32,
        };
        if ScreenToClient(hwnd, &mut point).as_bool() && corner(hwnd, point) {
            return Some(LRESULT(HTCLIENT as isize));
        }
    }
    if msg == WM_LBUTTONDOWN
        && corner(
            hwnd,
            POINT {
                x: lp.0 as i16 as i32,
                y: (lp.0 >> 16) as i16 as i32,
            },
        )
    {
        state.live_resume.set(true);
        return Some(LRESULT(0));
    }
    None
}
unsafe fn corner(hwnd: HWND, point: POINT) -> bool {
    let mut r = RECT::default();
    GetClientRect(hwnd, &mut r).is_ok()
        && point.x >= r.right - 16
        && point.y >= r.bottom - 16
        && point.x < r.right
        && point.y < r.bottom
        && (point.x >= r.right - 5 || point.y >= r.bottom - 5)
}
pub(super) unsafe fn draw(
    target: &windows::Win32::Graphics::Direct2D::ID2D1RenderTarget,
    brush: &windows::Win32::Graphics::Direct2D::ID2D1SolidColorBrush,
    size: windows::Win32::Graphics::Direct2D::Common::D2D_SIZE_U,
    dpi: u32,
) {
    use windows::Win32::Graphics::Direct2D::Common::*;
    let scale = 96.0 / dpi.max(1) as f32;
    brush.SetColor(&D2D1_COLOR_F {
        r: 217.0 / 255.0,
        g: 1.0,
        b: 56.0 / 255.0,
        a: 1.0,
    });
    target.DrawRectangle(
        &D2D_RECT_F {
            left: scale,
            top: scale,
            right: size.width as f32 * scale - scale,
            bottom: size.height as f32 * scale - scale,
        },
        brush,
        2.0 * scale,
        None,
    );
}

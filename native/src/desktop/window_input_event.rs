//! 本地Live与远程viewer共用Win32→归一化事件转换；此纯适配不授控制权或投递输入。
use crate::{
    live_input::{Button, Event},
    live_viewport::Viewport,
};
use windows::Win32::{Foundation::*, Graphics::Gdi::ScreenToClient, UI::WindowsAndMessaging::*};
pub(super) fn recognized(message: u32) -> bool {
    matches!(
        message,
        WM_KEYDOWN
            | WM_KEYUP
            | WM_SYSKEYDOWN
            | WM_SYSKEYUP
            | WM_MOUSEMOVE
            | WM_LBUTTONDOWN
            | WM_LBUTTONUP
            | WM_LBUTTONDBLCLK
            | WM_RBUTTONDOWN
            | WM_RBUTTONUP
            | WM_RBUTTONDBLCLK
            | WM_MBUTTONDOWN
            | WM_MBUTTONUP
            | WM_MBUTTONDBLCLK
            | WM_MOUSEWHEEL
            | WM_MOUSEHWHEEL
    )
}
pub(super) fn button(event: Event) -> Option<(u8, bool)> {
    if let Event::Button { button, down, .. } = event {
        Some((
            match button {
                Button::Left => 1,
                Button::Right => 2,
                Button::Middle => 4,
            },
            down,
        ))
    } else {
        None
    }
}
pub(super) unsafe fn decode(
    hwnd: HWND,
    message: u32,
    wp: WPARAM,
    lp: LPARAM,
    view: Viewport,
    held: bool,
) -> Result<Option<Event>, ()> {
    if matches!(message, WM_KEYDOWN | WM_KEYUP | WM_SYSKEYDOWN | WM_SYSKEYUP) {
        if !(1..=254).contains(&wp.0) {
            return Err(());
        }
        return Ok(Some(Event::Key {
            code: wp.0 as u16,
            down: matches!(message, WM_KEYDOWN | WM_SYSKEYDOWN),
        }));
    }
    if !recognized(message) {
        return Ok(None);
    }
    let mut point = POINT {
        x: lp.0 as u16 as i16 as i32,
        y: (lp.0 >> 16) as u16 as i16 as i32,
    };
    let wheel = matches!(message, WM_MOUSEWHEEL | WM_MOUSEHWHEEL);
    if wheel && !ScreenToClient(hwnd, &mut point).as_bool() {
        return Err(());
    }
    let up = matches!(message, WM_LBUTTONUP | WM_RBUTTONUP | WM_MBUTTONUP);
    let Some(point) = view.normalized(
        [f64::from(point.x), f64::from(point.y)],
        held && (up || message == WM_MOUSEMOVE),
    ) else {
        return Ok(None);
    };
    Ok(Some(if message == WM_MOUSEMOVE {
        Event::Move { point }
    } else if wheel {
        Event::Wheel {
            point,
            delta: (wp.0 >> 16) as u16 as i16 as i32,
            horizontal: message == WM_MOUSEHWHEEL,
        }
    } else {
        Event::Button {
            point,
            button: match message {
                WM_LBUTTONDOWN | WM_LBUTTONUP | WM_LBUTTONDBLCLK => Button::Left,
                WM_RBUTTONDOWN | WM_RBUTTONUP | WM_RBUTTONDBLCLK => Button::Right,
                _ => Button::Middle,
            },
            down: !up,
            double: matches!(
                message,
                WM_LBUTTONDBLCLK | WM_RBUTTONDBLCLK | WM_MBUTTONDBLCLK
            ),
        }
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn shared_mapper_rejects_letterbox_and_preserves_captured_release_and_keys() {
        let view = Viewport::fit([200, 100], [400, 400]).unwrap();
        unsafe {
            assert_eq!(
                decode(
                    HWND::default(),
                    WM_LBUTTONDOWN,
                    WPARAM(0),
                    LPARAM(50 << 16 | 200),
                    view,
                    false
                ),
                Ok(None)
            );
            assert_eq!(
                decode(
                    HWND::default(),
                    WM_LBUTTONUP,
                    WPARAM(0),
                    LPARAM(50 << 16 | 200),
                    view,
                    true
                ),
                Ok(Some(Event::Button {
                    point: [0.5, 0.0],
                    button: Button::Left,
                    down: false,
                    double: false
                }))
            );
            assert_eq!(
                decode(
                    HWND::default(),
                    WM_KEYDOWN,
                    WPARAM(65),
                    LPARAM(0),
                    view,
                    false
                ),
                Ok(Some(Event::Key {
                    code: 65,
                    down: true
                }))
            );
            assert!(decode(
                HWND::default(),
                WM_KEYDOWN,
                WPARAM(255),
                LPARAM(0),
                view,
                false
            )
            .is_err());
        }
    }
}

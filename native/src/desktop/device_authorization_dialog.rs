//! Validate service-address drafts only on confirmation; standard Win32 dialogs provide Tab, Enter, Escape, and focus restoration.
use windows::{
    core::HSTRING,
    Win32::{Foundation::*, UI::WindowsAndMessaging::*},
};
#[path = "device_authorization_dialog_template.rs"]
mod layout;
const USER: i32 = (2 * std::mem::size_of::<isize>()) as i32;
struct Input<'a> {
    initial: &'a str,
    result: Option<String>,
    art: bool,
    password: zeroize::Zeroizing<String>,
}
pub(super) unsafe fn show(
    owner: HWND,
    initial: &str,
) -> Option<(String, zeroize::Zeroizing<String>)> {
    show_input(owner, initial, false)
}
pub(super) unsafe fn show_art(owner: HWND) -> Option<String> {
    show_input(owner, "", true).map(|(value, _)| value)
}
unsafe fn show_input(
    owner: HWND,
    initial: &str,
    art: bool,
) -> Option<(String, zeroize::Zeroizing<String>)> {
    let template = layout::template(art);
    let mut input = Input {
        initial,
        result: None,
        art,
        password: zeroize::Zeroizing::new(String::new()),
    };
    if DialogBoxIndirectParamW(
        None,
        template.as_ptr().cast(),
        Some(owner),
        Some(procedure),
        LPARAM(&mut input as *mut _ as isize),
    ) == -1
    {
        eprintln!(
            "Authorization dialog failed: {}",
            windows::core::Error::from_win32()
        );
    }
    input.result.map(|value| (value, input.password))
}
unsafe extern "system" fn procedure(hwnd: HWND, message: u32, wp: WPARAM, lp: LPARAM) -> isize {
    if message == WM_INITDIALOG {
        SetWindowLongPtrW(hwnd, WINDOW_LONG_PTR_INDEX(USER), lp.0);
        let input = &*(lp.0 as *const Input<'_>);
        for id in [1, 2] {
            if let Ok(button) = GetDlgItem(Some(hwnd), id) {
                let _ = crate::desktop::dialog_controls::button(button);
            }
        }
        for id in [101, 102] {
            if let Ok(edit) = GetDlgItem(Some(hwnd), id) {
                let _ = crate::desktop::dialog_controls::edit(edit);
            }
        }
        if let Ok(edit) = GetDlgItem(Some(hwnd), 102) {
            let _ = ShowWindow(edit, if input.art { SW_HIDE } else { SW_SHOW });
            SendMessageW(edit, 0x00C5, Some(WPARAM(256)), Some(LPARAM(0)));
        }
        if let Ok(label) = GetDlgItem(Some(hwnd), 202) {
            let _ = ShowWindow(label, if input.art { SW_HIDE } else { SW_SHOW });
        }
        let _ = SetDlgItemTextW(hwnd, 101, &HSTRING::from(input.initial));
        if let Ok(edit) = GetDlgItem(Some(hwnd), 101) {
            SendMessageW(
                edit,
                0x00C5,
                Some(WPARAM(if input.art { 256 } else { 2048 })),
                Some(LPARAM(0)),
            );
            let _ = windows::Win32::UI::Input::KeyboardAndMouse::SetFocus(Some(edit));
        }
        place(hwnd);
        return 0;
    }
    if let Some(result) = crate::desktop::dialog_controls::background(message, wp, lp) {
        return result;
    }
    if message == WM_DRAWITEM {
        let item = &*(lp.0 as *const windows::Win32::UI::Controls::DRAWITEMSTRUCT);
        crate::desktop::dialog_controls::draw_button(item, item.CtlID == IDOK.0 as u32);
        return 1;
    }
    if message == WM_CLOSE || (message == WM_COMMAND && wp.0 & 0xffff == IDCANCEL.0 as usize) {
        let _ = EndDialog(hwnd, 0);
        return 1;
    }
    if message != WM_COMMAND || wp.0 & 0xffff != IDOK.0 as usize {
        return 0;
    }
    let input = GetWindowLongPtrW(hwnd, WINDOW_LONG_PTR_INDEX(USER)) as *mut Input<'_>;
    if input.is_null() {
        return 0;
    }
    let mut buffer = [0u16; 2050];
    let count = GetDlgItemTextW(hwnd, 101, &mut buffer) as usize;
    let value = String::from_utf16_lossy(&buffer[..count]);
    let validated = if (*input).art {
        crate::live_relay_viewer_identity::identifier(&value).then_some(value.clone())
    } else {
        crate::device_session_attempt::canonical_origin(&value).ok()
    };
    if let Some(origin) = validated {
        if !(*input).art {
            let mut secret = zeroize::Zeroizing::new([0u16; 257]);
            let count = GetDlgItemTextW(hwnd, 102, &mut *secret) as usize;
            (*input).password = zeroize::Zeroizing::new(String::from_utf16_lossy(&secret[..count]));
        }
        (*input).result = Some(origin);
        let _ = EndDialog(hwnd, 1);
    } else {
        let _ = SetDlgItemTextW(
            hwnd,
            103,
            &HSTRING::from(if (*input).art {
                "请输入有效的 Art ID。"
            } else {
                "请输入 HTTPS 服务地址；不能包含账号、路径、查询或片段。"
            }),
        );
    }
    1
}

unsafe fn place(hwnd: HWND) {
    use windows::Win32::Graphics::Gdi::*;
    let Ok(owner) = GetWindow(hwnd, GW_OWNER) else {
        return;
    };
    let (mut parent, mut window) = (RECT::default(), RECT::default());
    let mut monitor = MONITORINFO {
        cbSize: std::mem::size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };
    if GetWindowRect(owner, &mut parent).is_err()
        || GetWindowRect(hwnd, &mut window).is_err()
        || !GetMonitorInfoW(
            MonitorFromWindow(owner, MONITOR_DEFAULTTONEAREST),
            &mut monitor,
        )
        .as_bool()
    {
        return;
    }
    let work = monitor.rcWork;
    let width = window.right - window.left;
    let height = window.bottom - window.top;
    let x = (parent.left + (parent.right - parent.left - width) / 2)
        .clamp(work.left, (work.right - width).max(work.left));
    let y = (parent.top + (parent.bottom - parent.top - height) / 2)
        .clamp(work.top, (work.bottom - height).max(work.top));
    let _ = SetWindowPos(
        hwnd,
        None,
        x,
        y,
        0,
        0,
        SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn styled_connection_preserves_edit_focus_password_and_enter_validation() {
        unsafe {
            let owner = CreateWindowExW(
                WS_EX_TOOLWINDOW,
                windows::core::w!("STATIC"),
                None,
                WS_POPUP,
                300,
                200,
                400,
                300,
                None,
                None,
                None,
                None,
            )
            .unwrap();
            let mut input = Input {
                initial: "invalid",
                result: None,
                art: false,
                password: zeroize::Zeroizing::new(String::new()),
            };
            let template = layout::template(false);
            let hwnd = CreateDialogIndirectParamW(
                None,
                template.as_ptr().cast(),
                Some(owner),
                Some(procedure),
                LPARAM(&mut input as *mut _ as isize),
            )
            .unwrap();
            let edit = GetDlgItem(Some(hwnd), 101).unwrap();
            assert_eq!(GetWindowLongW(hwnd, GWL_STYLE) as u32 & WS_CAPTION.0, 0);
            for id in [1, 2] {
                let button = GetDlgItem(Some(hwnd), id).unwrap();
                assert_eq!(
                    GetWindowLongW(button, GWL_STYLE) as u32 & BS_TYPEMASK as u32,
                    BS_OWNERDRAW as u32
                );
            }
            assert!(
                GetWindowLongW(GetDlgItem(Some(hwnd), 102).unwrap(), GWL_STYLE) as u32
                    & ES_PASSWORD as u32
                    != 0
            );
            let _ = ShowWindow(hwnd, SW_SHOW);
            windows::Win32::UI::Input::KeyboardAndMouse::SetFocus(Some(edit)).unwrap();
            assert!(IsDialogMessageW(
                hwnd,
                &MSG {
                    hwnd: edit,
                    message: WM_KEYDOWN,
                    wParam: WPARAM(13),
                    ..Default::default()
                }
            )
            .as_bool());
            assert!(input.result.is_none());
            assert!(GetWindowTextLengthW(GetDlgItem(Some(hwnd), 103).unwrap()) > 0);
            DestroyWindow(hwnd).unwrap();
            DestroyWindow(owner).unwrap();
        }
    }
}

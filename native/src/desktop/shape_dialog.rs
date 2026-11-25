//! System modal input dialog; lend state only during synchronous DialogBox calls without Canvas borrows.
use windows::{
    core::HSTRING,
    Win32::{Foundation::*, UI::WindowsAndMessaging::*},
};

const INPUT: i32 = 101;
#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-canvas")))]
#[path = "shape_dialog_trigger_tests.rs"]
mod trigger_tests;
const ERROR: i32 = 102;
#[cfg(test)]
thread_local! {
    static FAIL_TRIGGER_AFFINITY: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}
// DWLP_USER = sizeof(LRESULT) + sizeof(DLGPROC); do not use 32-bit-only DWL_USER.
const DWLP_USER: i32 = (2 * std::mem::size_of::<isize>()) as i32;
#[derive(Clone, Copy, PartialEq)]
enum Mode {
    GroupName,
    Ratio,
    Step,
    Text,
    Parameters,
    Trigger,
    LocalTrigger,
    SerialRadius,
    Numeric(super::numeric_style::Kind),
}
struct InputState {
    value: String,
    mode: Mode,
    result: Option<String>,
}

pub(super) unsafe fn show(owner: HWND, value: String, ratio: bool) -> Option<String> {
    show_input(owner, value, if ratio { Mode::Ratio } else { Mode::Step })
}
pub(super) unsafe fn show_parameters(owner: HWND, value: String) -> Option<String> {
    show_input(owner, value, Mode::Parameters)
}
pub(super) unsafe fn show_trigger(owner: HWND, value: String) -> Option<String> {
    show_input(owner, value, Mode::Trigger)
}
pub(super) unsafe fn show_local_trigger(owner: HWND, value: String) -> Option<String> {
    show_input(owner, value, Mode::LocalTrigger)
}
pub(super) unsafe fn show_group_name(owner: HWND, value: String) -> Option<String> {
    show_input(owner, value, Mode::GroupName)
}
pub(super) unsafe fn show_serial_radius(owner: HWND, value: String) -> Option<String> {
    show_input(owner, value, Mode::SerialRadius)
}
pub(super) unsafe fn show_numeric(
    owner: HWND,
    value: String,
    kind: super::numeric_style::Kind,
) -> Option<String> {
    show_input(owner, value, Mode::Numeric(kind))
}
unsafe fn show_input(owner: HWND, value: String, mode: Mode) -> Option<String> {
    let mut state = InputState {
        value,
        mode,
        result: None,
    };
    let template = template(mode);
    let result = DialogBoxIndirectParamW(
        None,
        template.as_ptr().cast(),
        Some(owner),
        Some(procedure),
        LPARAM(&mut state as *mut _ as isize),
    );
    if result == -1 {
        eprintln!(
            "native-shape-dialog: {}",
            windows::core::Error::from_win32()
        );
    }
    state.result
}

unsafe fn protect_trigger(hwnd: HWND) -> windows::core::Result<()> {
    #[cfg(test)]
    if FAIL_TRIGGER_AFFINITY.with(|fail| fail.replace(false)) {
        return Err(windows::core::Error::new(
            E_FAIL,
            "injected trigger affinity failure",
        ));
    }
    SetWindowDisplayAffinity(hwnd, WDA_EXCLUDEFROMCAPTURE)
}

unsafe extern "system" fn procedure(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> isize {
    if message == WM_INITDIALOG {
        let state = &*(lparam.0 as *const InputState);
        if matches!(state.mode, Mode::Trigger | Mode::LocalTrigger) {
            if let Err(error) = protect_trigger(hwnd) {
                // EndDialog during initialization retires the modal before it can show or take focus.
                eprintln!("native-trigger-dialog: capture protection unavailable: {error}");
                let _ = EndDialog(hwnd, 0);
                return 0;
            }
        }
        SetWindowLongPtrW(hwnd, WINDOW_LONG_PTR_INDEX(DWLP_USER), lparam.0);
        let _ = SetDlgItemTextW(hwnd, INPUT, &HSTRING::from(&state.value));
        if let Ok(edit) = GetDlgItem(Some(hwnd), INPUT) {
            // Bound input so native edit controls cannot accumulate unbounded pasted content.
            SendMessageW(
                edit,
                0x00C5,
                Some(WPARAM(
                    if matches!(state.mode, Mode::Trigger | Mode::LocalTrigger) {
                        crate::live_relay_trigger_command::MAX_TEXT
                    } else if state.mode == Mode::Parameters {
                        super::capability_parameters::MAX_TEXT
                    } else if state.mode == Mode::GroupName {
                        256
                    } else if state.mode == Mode::Text {
                        crate::text_annotation::MAX_TEXT_UNITS
                    } else {
                        32
                    },
                )),
                Some(LPARAM(0)),
            ); // EM_SETLIMITTEXT
            SendMessageW(edit, 0x00B1, Some(WPARAM(0)), Some(LPARAM(-1))); // EM_SETSEL
        }
        return 1;
    }
    if message == WM_CLOSE {
        let _ = EndDialog(hwnd, 0);
        return 1;
    }
    if message != WM_COMMAND {
        return 0;
    }
    let command = wparam.0 & 0xffff;
    if command == IDCANCEL.0 as usize {
        let _ = EndDialog(hwnd, 0);
        return 1;
    }
    if command != IDOK.0 as usize {
        return 0;
    }
    let pointer = GetWindowLongPtrW(hwnd, WINDOW_LONG_PTR_INDEX(DWLP_USER)) as *mut InputState;
    if pointer.is_null() {
        return 0;
    }
    let state = &mut *pointer;
    let mut buffer = vec![
        0u16;
        if matches!(state.mode, Mode::Trigger | Mode::LocalTrigger) {
            crate::live_relay_trigger_command::MAX_TEXT + 1
        } else if state.mode == Mode::Parameters {
            super::capability_parameters::MAX_TEXT + 1
        } else if state.mode == Mode::GroupName {
            257
        } else if state.mode == Mode::Text {
            crate::text_annotation::MAX_TEXT_UNITS + 1
        } else {
            33
        }
    ];
    let count = GetDlgItemTextW(hwnd, INPUT, &mut buffer) as usize;
    let text = String::from_utf16_lossy(&buffer[..count]);
    let valid = if state.mode == Mode::Trigger {
        crate::live_relay_trigger_command::Command::parse(&text).is_ok()
    } else if state.mode == Mode::LocalTrigger {
        serde_json::from_str::<crate::live_relay_trigger_registration::Condition>(&text).is_ok_and(
            |condition| {
                condition.validate().is_ok()
                    && condition.revision <= crate::live_input::MAX_SEQUENCE
            },
        )
    } else if state.mode == Mode::Parameters {
        super::capability_parameters::parse(&text).is_some()
    } else if state.mode == Mode::GroupName {
        !text.is_empty() && text.len() <= 256 && !text.chars().any(char::is_control)
    } else if let Mode::Numeric(kind) = state.mode {
        kind.parse(&text).is_some()
    } else if state.mode == Mode::Text {
        crate::text_annotation::TextAnnotation::restored(&text, "微软雅黑", 16.0, false).is_some()
    } else if state.mode == Mode::Ratio {
        crate::shape_value::ratio(&text).is_some()
    } else if state.mode == Mode::SerialRadius {
        crate::shape_value::step(&text).is_some_and(|value| (8..=999).contains(&value))
    } else {
        crate::shape_value::step(&text).is_some()
    };
    if valid {
        state.result = Some(text);
        let _ = EndDialog(hwnd, 1);
    } else {
        let message = if matches!(state.mode, Mode::Trigger | Mode::LocalTrigger) {
            crate::ui_text::text("shape-trigger-invalid")
        } else if state.mode == Mode::Parameters {
            crate::ui_text::text("shape-json-invalid")
        } else if state.mode == Mode::GroupName {
            crate::ui_text::text("shape-name-invalid")
        } else if let Mode::Numeric(kind) = state.mode {
            kind.hint()
        } else if state.mode == Mode::Text {
            crate::ui_text::text("shape-text-invalid")
        } else if state.mode == Mode::Ratio {
            crate::ui_text::text("shape-ratio-invalid")
        } else if state.mode == Mode::SerialRadius {
            crate::ui_text::text("shape-radius-invalid")
        } else {
            crate::ui_text::text("shape-step-invalid")
        };
        let _ = SetDlgItemTextW(hwnd, ERROR, &HSTRING::from(message));
    }
    1
}

// DLGTEMPLATE items are DWORD-aligned; use Vec<u32> at the end to guarantee base-address alignment.
fn template(mode: Mode) -> Vec<u32> {
    let ratio = mode == Mode::Ratio;
    let text_input = matches!(
        mode,
        Mode::Text | Mode::Parameters | Mode::Trigger | Mode::LocalTrigger
    );
    fn string(data: &mut Vec<u16>, text: &str) {
        data.extend(text.encode_utf16());
        data.push(0);
    }
    fn dword(data: &mut Vec<u16>, value: u32) {
        data.extend([value as u16, (value >> 16) as u16]);
    }
    let mut data = Vec::new();
    dword(
        &mut data,
        WS_POPUP.0 | WS_CAPTION.0 | WS_SYSMENU.0 | DS_MODALFRAME as u32 | DS_SETFONT as u32,
    );
    dword(&mut data, 0);
    let extra = if text_input { 100 } else { 0 };
    data.extend([4, 0, 0, 340, 112 + extra, 0, 0]);
    string(
        &mut data,
        if mode == Mode::LocalTrigger {
            crate::ui_text::text("shape-local-trigger-title")
        } else if mode == Mode::Trigger {
            crate::ui_text::text("shape-trigger-title")
        } else if mode == Mode::Parameters {
            crate::ui_text::text("shape-parameters-title")
        } else if mode == Mode::GroupName {
            crate::ui_text::text("shape-group-title")
        } else if let Mode::Numeric(kind) = mode {
            kind.title()
        } else if text_input {
            crate::ui_text::text("shape-text-title")
        } else if ratio {
            crate::ui_text::text("shape-ratio-title")
        } else if mode == Mode::SerialRadius {
            crate::ui_text::text("shape-radius-title")
        } else {
            crate::ui_text::text("shape-step-title")
        },
    );
    data.push(9);
    string(&mut data, "Microsoft YaHei UI");
    for (id, class, style, x, y, width, height, text) in [
        (
            INPUT as u16,
            0x81,
            WS_BORDER.0
                | WS_TABSTOP.0
                | if text_input {
                    WS_VSCROLL.0
                        | ES_MULTILINE as u32
                        | ES_WANTRETURN as u32
                        | ES_AUTOVSCROLL as u32
                } else {
                    ES_AUTOHSCROLL as u32
                },
            10,
            12,
            320,
            16 + extra,
            "",
        ),
        (
            ERROR as u16,
            0x82,
            0,
            10,
            34 + extra,
            320,
            40,
            if mode == Mode::LocalTrigger {
                crate::ui_text::text("shape-local-trigger-help")
            } else if mode == Mode::Trigger {
                crate::ui_text::text("shape-trigger-help")
            } else if mode == Mode::Parameters {
                crate::ui_text::text("shape-parameters-help")
            } else if mode == Mode::GroupName {
                crate::ui_text::text("shape-group-title")
            } else if let Mode::Numeric(kind) = mode {
                kind.hint()
            } else if text_input {
                crate::ui_text::text("shape-text-help")
            } else if ratio {
                crate::ui_text::text("shape-ratio-help")
            } else if mode == Mode::SerialRadius {
                crate::ui_text::text("shape-radius-help")
            } else {
                crate::ui_text::text("shape-step-help")
            },
        ),
        (
            IDOK.0 as u16,
            0x80,
            WS_TABSTOP.0 | BS_DEFPUSHBUTTON as u32,
            140,
            84 + extra,
            130,
            18,
            if mode == Mode::LocalTrigger {
                crate::ui_text::text("shape-allow-callback")
            } else {
                crate::ui_text::text("common-ok")
            },
        ),
        (
            IDCANCEL.0 as u16,
            0x80,
            WS_TABSTOP.0,
            275,
            84 + extra,
            55,
            18,
            crate::ui_text::text("common-cancel"),
        ),
    ] {
        if data.len() % 2 != 0 {
            data.push(0);
        }
        dword(&mut data, WS_CHILD.0 | WS_VISIBLE.0 | style);
        dword(&mut data, 0);
        data.extend([x, y, width, height, id, 0xffff, class]);
        string(&mut data, text);
        data.push(0);
    }
    if data.len() % 2 != 0 {
        data.push(0);
    }
    data.chunks_exact(2)
        .map(|words| words[0] as u32 | ((words[1] as u32) << 16))
        .collect()
}

#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-canvas")))]
#[path = "shape_dialog_tests.rs"]
mod tests;

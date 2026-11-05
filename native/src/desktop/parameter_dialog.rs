//! 单个公共参数的原生输入框；模态期间只拥有独立值，不借用应用或文档。
use super::parameter_value::{Spec, MAX_TEXT};
use serde_json::Value;
use windows::{
    core::HSTRING,
    Win32::{Foundation::*, UI::WindowsAndMessaging::*},
};
const INPUT: i32 = 101;
const ERROR: i32 = 102;
const DOWN: usize = 103;
const UP: usize = 104;
const USER: i32 = (2 * std::mem::size_of::<isize>()) as i32;
struct Input {
    slider: Option<super::parameter_slider::Control>,
    spec: Spec,
    text: String,
    result: Option<Value>,
}
pub(super) unsafe fn show(owner: HWND, spec: Spec, text: String) -> Option<Value> {
    let template = template(&spec);
    let mut input = Input {
        slider: None,
        spec,
        text,
        result: None,
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
            "Parameter dialog failed: {}",
            windows::core::Error::from_win32()
        );
    }
    input.result
}
unsafe extern "system" fn procedure(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> isize {
    if message == WM_INITDIALOG {
        SetWindowLongPtrW(hwnd, WINDOW_LONG_PTR_INDEX(USER), lparam.0);
        let input = &*(lparam.0 as *const Input);
        let _ = SetDlgItemTextW(hwnd, INPUT, &HSTRING::from(&input.text));
        let _ = SetDlgItemTextW(hwnd, ERROR, &HSTRING::from(input.spec.hint()));
        if let Ok(edit) = GetDlgItem(Some(hwnd), INPUT) {
            SendMessageW(edit, 0x00C5, Some(WPARAM(MAX_TEXT)), Some(LPARAM(0)));
            SendMessageW(edit, 0x00B1, Some(WPARAM(0)), Some(LPARAM(-1)));
        }
        let range = input.spec.slider(&input.text);
        let extra = if input.spec.multiline { 80 } else { 0 };
        if let Some(range) = range {
            let mut rect = RECT {
                left: 10,
                top: 30 + extra,
                right: 250,
                bottom: 48 + extra,
            };
            if MapDialogRect(hwnd, &mut rect).is_ok() {
                match super::parameter_slider::Control::create(hwnd, rect, range) {
                    Ok(slider) => (*(lparam.0 as *mut Input)).slider = Some(slider),
                    Err(error) => eprintln!("Parameter slider unavailable: {error}"),
                }
            }
        }
        return 1;
    }
    if message == WM_HSCROLL
        || message == WM_COMMAND
            && (wparam.0 & 0xffff) == INPUT as usize
            && (wparam.0 >> 16) == EN_CHANGE as usize
    {
        let pointer = GetWindowLongPtrW(hwnd, WINDOW_LONG_PTR_INDEX(USER)) as *mut Input;
        if pointer.is_null() {
            return 0;
        }
        if message == WM_HSCROLL {
            let text = (*pointer)
                .slider
                .as_ref()
                .and_then(|slider| slider.changed(lparam));
            if let Some(text) = text {
                let _ = SetDlgItemTextW(hwnd, INPUT, &HSTRING::from(text));
            }
        } else {
            let mut buffer = vec![0; MAX_TEXT + 1];
            let count = GetDlgItemTextW(hwnd, INPUT, &mut buffer) as usize;
            let range = String::from_utf16(&buffer[..count])
                .ok()
                .and_then(|text| (*pointer).spec.slider(&text));
            if let (Some(slider), Some(range)) = ((*pointer).slider.as_mut(), range) {
                slider.sync(range);
            }
        }
        return 1;
    }
    if message == WM_CLOSE || message == WM_COMMAND && wparam.0 & 0xffff == IDCANCEL.0 as usize {
        let _ = EndDialog(hwnd, 0);
        return 1;
    }
    let command = wparam.0 & 0xffff;
    if message != WM_COMMAND || ![IDOK.0 as usize, DOWN, UP].contains(&command) {
        return 0;
    }
    let pointer = GetWindowLongPtrW(hwnd, WINDOW_LONG_PTR_INDEX(USER)) as *mut Input;
    if pointer.is_null() {
        return 0;
    }
    let input = &mut *pointer;
    let mut buffer = vec![0; MAX_TEXT + 1];
    let count = GetDlgItemTextW(hwnd, INPUT, &mut buffer) as usize;
    if command == DOWN || command == UP {
        if let Some(text) = String::from_utf16(&buffer[..count])
            .ok()
            .and_then(|text| input.spec.stepped(&text, command == UP))
        {
            let _ = SetDlgItemTextW(hwnd, INPUT, &HSTRING::from(text));
        }
        return 1;
    }
    let value = String::from_utf16(&buffer[..count])
        .ok()
        .and_then(|text| input.spec.parse(&text));
    if let Some(value) = value {
        input.result = Some(value);
        let _ = EndDialog(hwnd, 1);
    } else {
        let _ = SetDlgItemTextW(
            hwnd,
            ERROR,
            &HSTRING::from(format!("输入无效。{}", input.spec.hint())),
        );
    }
    1
}
fn template(spec: &Spec) -> Vec<u32> {
    fn string(data: &mut Vec<u16>, text: &str) {
        data.extend(text.encode_utf16());
        data.push(0);
    }
    fn dword(data: &mut Vec<u16>, value: u32) {
        data.extend([value as u16, (value >> 16) as u16]);
    }
    let extra = if spec.multiline { 80 } else { 0 };
    let slider_extra = if spec.number { 26 } else { 0 };
    let mut data = Vec::new();
    dword(
        &mut data,
        WS_POPUP.0 | WS_CAPTION.0 | WS_SYSMENU.0 | DS_MODALFRAME as u32 | DS_SETFONT as u32,
    );
    dword(&mut data, 0);
    data.extend([
        if spec.number { 6 } else { 4 },
        0,
        0,
        260,
        100 + extra + slider_extra,
        0,
        0,
    ]);
    string(&mut data, &spec.title);
    data.push(9);
    string(&mut data, "Microsoft YaHei UI");
    let mut controls = vec![
        (
            INPUT as u16,
            0x81,
            WS_BORDER.0
                | WS_TABSTOP.0
                | if spec.multiline {
                    WS_VSCROLL.0
                        | ES_MULTILINE as u32
                        | ES_WANTRETURN as u32
                        | ES_AUTOVSCROLL as u32
                } else {
                    ES_AUTOHSCROLL as u32
                },
            10,
            10,
            if spec.number { 180 } else { 240 },
            16 + extra,
            "",
        ),
        (
            ERROR as u16,
            0x82,
            0,
            10,
            32 + extra + slider_extra,
            240,
            28,
            "",
        ),
        (
            IDOK.0 as u16,
            0x80,
            WS_TABSTOP.0 | BS_DEFPUSHBUTTON as u32,
            130,
            72 + extra + slider_extra,
            55,
            18,
            "确定",
        ),
        (
            IDCANCEL.0 as u16,
            0x80,
            WS_TABSTOP.0,
            195,
            72 + extra + slider_extra,
            55,
            18,
            "取消",
        ),
    ];
    if spec.number {
        controls.extend([
            (DOWN as u16, 0x80, WS_TABSTOP.0, 195, 10, 25, 16, "−"),
            (UP as u16, 0x80, WS_TABSTOP.0, 225, 10, 25, 16, "+"),
        ]);
    }
    for (id, class, style, x, y, width, height, text) in controls {
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
        .map(|v| v[0] as u32 | ((v[1] as u32) << 16))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn hidden_dialog_controls_share_draft_without_committing() {
        let raw = serde_json::json!({"id":"art","parameters":[{"id":"value","type":"number","min":0,"max":1,"step":0.1}]});
        let capability = crate::loom_capability::Capability::new(&raw).unwrap();
        let spec = Spec::from_parameter(&capability.parameters().next().unwrap()).unwrap();
        let template = template(&spec);
        let mut input = Input {
            slider: None,
            spec,
            text: "0.2".into(),
            result: None,
        };
        unsafe {
            // 模板不含 WS_VISIBLE；仅验证自己创建的隐藏对话框，不打开用户模态界面。
            let hwnd = CreateDialogIndirectParamW(
                None,
                template.as_ptr().cast(),
                None,
                Some(procedure),
                LPARAM(&mut input as *mut _ as isize),
            )
            .unwrap();
            assert!(!IsWindowVisible(hwnd).as_bool());
            let slider = GetDlgItem(Some(hwnd), super::super::parameter_slider::ID).unwrap();
            SendMessageW(
                slider,
                windows::Win32::UI::Controls::TBM_SETPOS,
                Some(WPARAM(0)),
                Some(LPARAM(3000)),
            );
            SendMessageW(hwnd, WM_HSCROLL, None, Some(LPARAM(slider.0 as isize)));
            let mut buffer = [0; 64];
            let count = GetDlgItemTextW(hwnd, INPUT, &mut buffer) as usize;
            assert_eq!(String::from_utf16(&buffer[..count]).unwrap(), "0.3");
            SendMessageW(hwnd, WM_COMMAND, Some(WPARAM(UP)), None);
            let count = GetDlgItemTextW(hwnd, INPUT, &mut buffer) as usize;
            assert_eq!(String::from_utf16(&buffer[..count]).unwrap(), "0.4");
            assert_eq!(SendMessageW(slider, WM_USER, None, None).0, 4000);
            assert!(input.result.is_none());
            DestroyWindow(hwnd).unwrap();
            assert!(!IsWindow(Some(slider)).as_bool());
        }
    }
}

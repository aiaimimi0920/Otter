//! 原生触发JSON模态框的真实控件验证；仅自有对话框消息，不向用户程序注入输入。
use super::*;
const VALID: &str = r#"{"bindingId":"b","enabled":false,"target":{"surfaceInstanceId":"s","surfaceAttachmentId":"a","surfaceNodeId":"n","surfaceEvent":"click","surfaceAction":"x"},"condition":{"conditionId":"c","revision":2,"observationId":"o","operator":"equals","operand":{"path":"/value/text","value":"完成"},"stableForMs":1000,"risingEdge":false,"rearm":true,"minimumConfidence":"exact"}}"#;
unsafe extern "system" fn test_proc(hwnd: HWND, message: u32, wp: WPARAM, lp: LPARAM) -> isize {
    let result = procedure(hwnd, message, wp, lp);
    if message == WM_INITDIALOG {
        let mut affinity = 0;
        GetWindowDisplayAffinity(hwnd, &mut affinity).unwrap();
        assert_eq!(affinity, WDA_EXCLUDEFROMCAPTURE.0);
        let _ = PostMessageW(Some(hwnd), WM_COMMAND, WPARAM(IDOK.0 as usize), LPARAM(0));
        let _ = PostMessageW(
            Some(hwnd),
            WM_COMMAND,
            WPARAM(IDCANCEL.0 as usize),
            LPARAM(0),
        );
    }
    result
}
#[test]
fn actual_trigger_dialog_accepts_complete_config_and_rejects_generic_json() {
    for (text, accepted) in [(VALID, true), ("{}", false), ("[]", false)] {
        let mut state = InputState {
            value: text.into(),
            mode: Mode::Trigger,
            result: None,
        };
        let data = template(Mode::Trigger);
        let result = unsafe {
            DialogBoxIndirectParamW(
                None,
                data.as_ptr().cast(),
                None,
                Some(test_proc),
                LPARAM(&mut state as *mut _ as isize),
            )
        };
        assert_ne!(result, -1);
        assert_eq!(state.result.as_deref(), accepted.then_some(text));
    }
}

#[test]
fn hidden_local_trigger_controls_require_explicit_auto_consent_and_strict_condition() {
    for accepted in [true, false] {
        let value = if accepted {
            serde_json::to_string(&crate::art_live_trigger::tests::condition()).unwrap()
        } else {
            "{}".into()
        };
        let mut state = InputState {
            value: value.clone(),
            mode: Mode::LocalTrigger,
            result: None,
        };
        let data = template(state.mode);
        unsafe {
            // 非模态隐藏自有控件，不运行会显示/激活的DialogBox测试。
            unsafe extern "system" fn hidden(
                hwnd: HWND,
                message: u32,
                wp: WPARAM,
                lp: LPARAM,
            ) -> isize {
                let value = procedure(hwnd, message, wp, lp);
                if message == WM_INITDIALOG {
                    0
                } else {
                    value
                }
            }
            let hwnd = CreateDialogIndirectParamW(
                None,
                data.as_ptr().cast(),
                None,
                Some(hidden),
                LPARAM(&mut state as *mut _ as isize),
            )
            .unwrap();
            assert!(!IsWindowVisible(hwnd).as_bool());
            let mut affinity = 0;
            GetWindowDisplayAffinity(hwnd, &mut affinity).unwrap();
            assert_eq!(affinity, WDA_EXCLUDEFROMCAPTURE.0);
            let mut text = [0u16; 64];
            let length = GetDlgItemTextW(hwnd, IDOK.0, &mut text) as usize;
            assert_eq!(String::from_utf16_lossy(&text[..length]), "允许自动回调");
            procedure(hwnd, WM_COMMAND, WPARAM(IDOK.0 as usize), LPARAM(0));
            assert_eq!(state.result, accepted.then_some(value));
            DestroyWindow(hwnd).unwrap();
        }
    }
}

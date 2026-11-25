//! Actual native trigger-JSON modal controls; only owned-dialog messages, without injecting input into user applications.
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
fn affinity_failure_retires_modal_before_draft_or_submit_is_available() {
    use std::cell::Cell;
    thread_local! {
        static INIT: Cell<Option<(bool, bool, bool)>> = const { Cell::new(None) };
        static SHOWN: Cell<bool> = const { Cell::new(false) };
    }
    unsafe extern "system" fn observe(hwnd: HWND, message: u32, wp: WPARAM, lp: LPARAM) -> isize {
        let result = procedure(hwnd, message, wp, lp);
        if message == WM_INITDIALOG {
            let empty =
                GetDlgItem(Some(hwnd), INPUT).is_ok_and(|edit| GetWindowTextLengthW(edit) == 0);
            let unattached = GetWindowLongPtrW(hwnd, WINDOW_LONG_PTR_INDEX(DWLP_USER)) == 0;
            let blocked = procedure(hwnd, WM_COMMAND, WPARAM(IDOK.0 as usize), LPARAM(0)) == 0;
            INIT.with(|value| value.set(Some((empty, unattached, blocked))));
            // Prevent a broken regression from leaving a visible dialog waiting for input.
            let _ = PostMessageW(Some(hwnd), WM_CLOSE, WPARAM(0), LPARAM(0));
        } else if message == WM_SHOWWINDOW && wp.0 != 0 {
            SHOWN.with(|value| value.set(true));
        }
        result
    }
    for mode in [Mode::Trigger, Mode::LocalTrigger] {
        let mut state = InputState {
            value: VALID.into(),
            mode,
            result: None,
        };
        let data = template(mode);
        FAIL_TRIGGER_AFFINITY.with(|fail| fail.set(true));
        let result = unsafe {
            DialogBoxIndirectParamW(
                None,
                data.as_ptr().cast(),
                None,
                Some(observe),
                LPARAM(&mut state as *mut _ as isize),
            )
        };
        assert_eq!(result, 0);
        assert_eq!(INIT.with(Cell::take), Some((true, true, true)));
        assert!(!SHOWN.with(Cell::take));
        assert!(!FAIL_TRIGGER_AFFINITY.with(Cell::get));
        assert!(state.result.is_none());
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
            // Modeless hidden owned controls only; do not run DialogBox tests that display/activate windows.
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

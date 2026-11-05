//! 通用原生输入对话框的既有验证矩阵；独立测试叶，不改变原模块测试身份。
use super::*;
#[test]
fn group_name_dialog_accepts_unicode_and_rejects_empty_or_byte_overflow() {
    for (value, accepted) in [
        ("测试分组".to_owned(), true),
        (String::new(), false),
        ("中".repeat(86), false),
    ] {
        let mut state = InputState {
            value: value.clone(),
            mode: Mode::GroupName,
            result: None,
        };
        let data = template(state.mode);
        let result = unsafe {
            DialogBoxIndirectParamW(
                None,
                data.as_ptr().cast(),
                None,
                Some(test_procedure),
                LPARAM(&mut state as *mut _ as isize),
            )
        };
        assert_ne!(result, -1);
        assert_eq!(state.result, accepted.then_some(value));
    }
}
unsafe extern "system" fn test_procedure(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> isize {
    let result = procedure(hwnd, message, wparam, lparam);
    if message == WM_INITDIALOG {
        // 无用户输入：先提交，若验证失败则由随后排队的取消关闭，避免测试悬挂。
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
fn native_parameter_dialog_accepts_objects_without_coercing_json_types() {
    for (value, accepted) in [
        (r#"{"文字":"测试","number":2,"empty":null}"#, true),
        ("{}", true),
        ("[]", false),
        ("bad", false),
    ] {
        let mut state = InputState {
            value: value.into(),
            mode: Mode::Parameters,
            result: None,
        };
        let data = template(state.mode);
        let result = unsafe {
            DialogBoxIndirectParamW(
                None,
                data.as_ptr().cast(),
                None,
                Some(test_procedure),
                LPARAM(&mut state as *mut _ as isize),
            )
        };
        assert_ne!(result, -1);
        assert_eq!(state.result.as_deref(), accepted.then_some(value));
    }
}
#[test]
fn native_numeric_dialog_validates_decimal_values_before_returning() {
    use super::super::numeric_style::Kind;
    for (kind, value, accepted) in [
        (Kind::Width, "12.5", true),
        (Kind::Width, "NaN", false),
        (Kind::EraserWidth, "4.25", true),
        (Kind::EraserWidth, "3.9", false),
        (Kind::EffectWidth, "200", true),
        (Kind::EffectWidth, "3.99", false),
        (Kind::MosaicCell, "2.4", true),
        (Kind::MosaicCell, "64.1", false),
        (Kind::Opacity, "0", true),
        (Kind::Opacity, "100.1", false),
        (Kind::BlurStrength, "2.4", true),
        (Kind::BlurStrength, "64.1", false),
        (Kind::BlurDefaultStrength, "2.4", true),
        (Kind::BlurDefaultStrength, "1.99", false),
    ] {
        let mut state = InputState {
            value: value.into(),
            mode: Mode::Numeric(kind),
            result: None,
        };
        let data = template(state.mode);
        let result = unsafe {
            DialogBoxIndirectParamW(
                None,
                data.as_ptr().cast(),
                None,
                Some(test_procedure),
                LPARAM(&mut state as *mut _ as isize),
            )
        };
        assert_ne!(result, -1);
        assert_eq!(
            state.result.as_deref(),
            if accepted { Some(value) } else { None }
        );
    }
}
#[test]
fn native_serial_radius_dialog_accepts_only_supported_range() {
    for (value, accepted) in [("7", false), ("8", true), ("999", true), ("1000", false)] {
        let mut state = InputState {
            value: value.into(),
            mode: Mode::SerialRadius,
            result: None,
        };
        let data = template(state.mode);
        let result = unsafe {
            DialogBoxIndirectParamW(
                None,
                data.as_ptr().cast(),
                None,
                Some(test_procedure),
                LPARAM(&mut state as *mut _ as isize),
            )
        };
        assert_ne!(result, -1);
        assert_eq!(
            state.result.as_deref(),
            if accepted { Some(value) } else { None }
        );
    }
}
#[test]
fn native_dialog_template_accepts_valid_input_and_rejects_invalid_input() {
    for (text, ratio, text_input, accepted) in [
        ("2.5:1", true, false, true),
        ("0:1", true, false, false),
        ("7", false, false, true),
        ("-1", false, false, false),
        ("你好 Otter\r\n第二行 🦦", false, true, true),
        // 编辑恢复允许原文为空/空白；新建文字仍由text_input的new构造拒绝。
        (" \r\n ", false, true, true),
        ("", false, true, true),
    ] {
        let mut state = InputState {
            value: text.into(),
            mode: if text_input {
                Mode::Text
            } else if ratio {
                Mode::Ratio
            } else {
                Mode::Step
            },
            result: None,
        };
        let data = template(state.mode);
        let result = unsafe {
            DialogBoxIndirectParamW(
                None,
                data.as_ptr().cast(),
                None,
                Some(test_procedure),
                LPARAM(&mut state as *mut _ as isize),
            )
        };
        assert_ne!(result, -1, "dialog template must be accepted by Windows");
        assert_eq!(
            state.result.as_deref(),
            if accepted { Some(text) } else { None }
        );
    }
}

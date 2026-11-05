//! 原生确认边界：取消为默认选择，不可信字段只显示为转义文本，返回后重新授权。
use super::*;
use crate::surface_confirmation::{Choice, Decision, Risk};
use windows::{
    core::{w, HSTRING},
    Win32::UI::WindowsAndMessaging::{
        MessageBoxW, IDCANCEL, IDNO, IDYES, MB_DEFBUTTON3, MB_ICONWARNING, MB_YESNOCANCEL,
    },
};

// MessageBox 没有适合长文审阅的滚动区域；拒绝超长内容，而非截掉风险信息后允许批准。
const MAX_DIALOG_UNITS: usize = 8192;

impl AuthorizedWindow {
    /// 调用方负责模态互斥与退出状态；本入口不发网络请求、不自动移交 owner。
    pub fn prompt_decision(&mut self, receipt: &mut EventReceipt) -> Result<Option<Decision>> {
        let hwnd = self.window.dialog_owner()?;
        self.prompt_decision_with(receipt, super::super::surface_windows::now_ms, |text| {
            native_choice(hwnd, text)
        })
    }
    /// 返回明确选择而不消费提示；调用方复核应用状态后才准备决定和移交 owner。
    pub fn prompt_batch_choice(
        &mut self,
        receipt: &BatchReceipt,
        index: usize,
    ) -> Result<Option<Choice>> {
        let hwnd = self.window.dialog_owner()?;
        self.prompt_batch_choice_with(
            receipt,
            index,
            super::super::surface_windows::now_ms,
            |text| native_choice(hwnd, text),
        )
    }
    pub(crate) fn prompt_batch_choice_with(
        &mut self,
        receipt: &BatchReceipt,
        index: usize,
        mut clock: impl FnMut() -> Result<u64>,
        choose: impl FnOnce(&str) -> Result<Option<Choice>>,
    ) -> Result<Option<Choice>> {
        let request = self.batch_confirmation_for(receipt, index, clock()?)?;
        let text = request_text(self.binding().origin(), request)?;
        let choice = choose(&text)?;
        self.batch_confirmation_for(receipt, index, clock()?)?;
        Ok(choice)
    }
    pub(crate) fn prompt_decision_with(
        &mut self,
        receipt: &mut EventReceipt,
        mut clock: impl FnMut() -> Result<u64>,
        choose: impl FnOnce(&str) -> Result<Option<Choice>>,
    ) -> Result<Option<Decision>> {
        let request = self.confirmation_for(receipt, clock()?)?;
        let text = request_text(self.binding().origin(), request)?;
        let choice = choose(&text)?;
        // 模态消息泵可能处理撤权/关闭，时间也可能跨过提示期限，不能复用展示前的许可。
        let now = clock()?;
        self.confirmation_for(receipt, now)?;
        choice
            .map(|choice| self.prepare_decision(receipt, choice, now))
            .transpose()
    }
}
pub(in crate::desktop) fn native_choice(
    hwnd: windows::Win32::Foundation::HWND,
    text: &str,
) -> Result<Option<Choice>> {
    let response = unsafe {
        MessageBoxW(
            Some(hwnd),
            &HSTRING::from(text),
            w!("确认远端操作"),
            MB_YESNOCANCEL | MB_ICONWARNING | MB_DEFBUTTON3,
        )
    };
    match response {
        IDYES => Ok(Some(Choice::Approve)),
        IDNO => Ok(Some(Choice::Reject)),
        IDCANCEL => Ok(None),
        _ => Err(Error::from_win32()),
    }
}
fn request_text(origin: &str, request: &crate::surface_confirmation::Request) -> Result<String> {
    let risk = match request.risk() {
        Risk::Low => "低",
        Risk::Medium => "中",
        Risk::High => "高",
    };
    dialog_text(
        origin,
        request.action(),
        risk,
        request.payload().map(|value| value.get()),
    )
}
fn escaped(value: &str) -> String {
    let mut text = String::with_capacity(value.len());
    for ch in value.chars() {
        if ch.is_control()
            || matches!(ch, '\u{061c}' | '\u{200e}' | '\u{200f}' | '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}')
        {
            text.extend(ch.escape_unicode());
        } else {
            text.push(ch);
        }
    }
    text
}
pub(in crate::desktop) fn dialog_text(
    origin: &str,
    action: &str,
    risk: &str,
    payload: Option<&str>,
) -> Result<String> {
    let text = format!("是否允许此远端操作？\n\n服务：{}\n操作：{}\n风险：{}\n\n远端提供的参数（不可信数据）：\n{}\n\n是：批准；否：拒绝；取消：暂不决定。",
        escaped(origin), escaped(action), risk, escaped(payload.unwrap_or("未提供")));
    if text.encode_utf16().count() > MAX_DIALOG_UNITS {
        return Err(Error::new(
            E_FAIL,
            "Surface confirmation exceeds native dialog display limit",
        ));
    }
    Ok(text)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn untrusted_controls_cannot_rewrite_dialog_labels() {
        let text = dialog_text(
            "https://localhost",
            "run\n风险：低\0\u{202e}",
            "高",
            Some("{\"x\":1}"),
        )
        .unwrap();
        assert!(text.contains("风险：高"));
        assert!(!text.contains("run\n风险"));
        assert!(!text.contains('\0'));
        assert!(!text.contains('\u{202e}'));
        assert!(text.contains("{\"x\":1}"));
    }
    #[test]
    fn oversized_prompt_is_not_silently_truncated() {
        assert!(dialog_text(
            "origin",
            "action",
            "高",
            Some(&"x".repeat(MAX_DIALOG_UNITS))
        )
        .is_err());
    }
}

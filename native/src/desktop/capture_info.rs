//! 按需查看当前文档的捕获元数据；无文件访问、常驻控件或后台刷新。
use super::*;
use crate::{capture_dynamic::Reason, desktop::canvas::CaptureSource};
pub(in crate::desktop) const COMMAND: usize = 0x1068;
pub(in crate::desktop) unsafe fn append(menu: HMENU, state: &State) -> Result<()> {
    let status = state
        .canvas
        .borrow()
        .capture_source()
        .map(|source| source.downgraded);
    AppendMenuW(
        menu,
        MF_STRING
            | if status.is_some() {
                MF_ENABLED
            } else {
                MF_GRAYED
            },
        COMMAND,
        if status == Some(true) {
            w!("截图信息（HDR 已降级）…")
        } else {
            w!("截图信息…")
        },
    )
}
pub(in crate::desktop) unsafe fn show(hwnd: HWND, state: &State) {
    if state.modals.blocked() || state.file_dialog.get() {
        return;
    }
    let text = state.canvas.borrow().capture_source().map(describe);
    let Some(text) = text else {
        return;
    };
    // MessageBox 会重入消息循环，不能跨调用借用画布；使用原有模态 guard。
    let Some(_guard) = super::super::file_dialog::Guard::begin(&state.file_dialog) else {
        return;
    };
    MessageBoxW(
        Some(hwnd),
        &HSTRING::from(text),
        w!("截图信息"),
        MB_OK | MB_ICONINFORMATION,
    );
}
fn reason(value: Option<Reason>) -> &'static str {
    match value {
        Some(Reason::ExplicitSdr) => "指定 SDR",
        Some(Reason::Workload) => "当前捕获类型使用 SDR",
        Some(Reason::CrossMonitor) => "选区跨越显示器",
        Some(Reason::DisplayUnknown) => "无法确认显示器 HDR 状态",
        Some(Reason::DisplaySdr) => "显示器未启用 HDR",
        Some(Reason::UnsupportedOs) => "当前系统不支持此 HDR 捕获路径",
        Some(Reason::HdrFailed) => "HDR 捕获失败",
        Some(Reason::AutoSdr) => "内容未超过 SDR 白电平",
        None => "—",
    }
}
// 路径/诊断仅作显示，控制字符不得伪造行标签；按 Unicode 字符截断，不切断 UTF-8。
fn single_line(value: &str) -> String {
    let mut chars = value.chars();
    let mut output: String = chars
        .by_ref()
        .take(1024)
        .map(|c| {
            if c.is_control() || matches!(c, '\u{2028}' | '\u{2029}') {
                ' '
            } else {
                c
            }
        })
        .collect();
    if chars.next().is_some() {
        output.push('…');
    }
    output
}
fn describe(source: &CaptureSource) -> String {
    let mut text = format!(
        "原图：{}\n画布预览：SDR\nHDR 降级：{}\n原因：{}",
        source.original_label(),
        if source.session_meta.as_ref().is_some_and(|meta| meta
            .get("downgradedFromHdr")
            .and_then(serde_json::Value::as_bool)
            .is_none())
        {
            "未记录"
        } else if source.downgraded {
            "是"
        } else {
            "否"
        },
        reason(source.reason)
    );
    if let Some(path) = &source.original {
        text.push_str(&format!(
            "\n原图路径：{}",
            single_line(&path.to_string_lossy())
        ));
    }
    if let Some(display) = source.display {
        text.push_str(&format!(
            "\n显示器 HDR：{}\nSDR 白电平：{:.1} nit{}\n显示器亮度范围：{:.3}–{:.1} nit",
            if !display.configuration_known {
                "未确认"
            } else if display.hdr_enabled {
                "启用"
            } else {
                "关闭"
            },
            display.sdr_white_nits,
            if display.white_level_known {
                ""
            } else {
                "（回退值）"
            },
            display.minimum_nits,
            display.maximum_nits
        ));
    }
    if let Some(diagnostic) = &source.diagnostic {
        text.push_str(&format!("\n诊断：{}", single_line(diagnostic)));
    }
    text
}
#[cfg(test)]
#[path = "capture_info_tests.rs"]
mod tests;

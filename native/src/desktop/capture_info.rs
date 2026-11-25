//! On-demand capture metadata for the current document; no file access, persistent controls, or background refresh.
use super::*;
use crate::ui_text::{format as message, text as tr};
use crate::{capture_dynamic::Reason, desktop::canvas::CaptureSource};
pub(in crate::desktop) const COMMAND: usize = 0x1068;
#[cfg(test)]
unsafe fn append(menu: HMENU, state: &State) -> Result<()> {
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
        &HSTRING::from(if status == Some(true) {
            tr("capture-info-menu-downgraded")
        } else {
            tr("capture-info-menu")
        }),
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
    // MessageBox reenters the message loop; do not borrow Canvas across the call, and use the existing modal guard.
    let Some(_guard) = super::super::file_dialog::Guard::begin(&state.file_dialog) else {
        return;
    };
    MessageBoxW(
        Some(hwnd),
        &HSTRING::from(text),
        &HSTRING::from(tr("capture-info-title")),
        MB_OK | MB_ICONINFORMATION,
    );
}
fn reason(value: Option<Reason>) -> &'static str {
    match value {
        Some(Reason::ExplicitSdr) => tr("capture-info-explicit-sdr"),
        Some(Reason::Workload) => tr("capture-info-workload-sdr"),
        Some(Reason::CrossMonitor) => tr("capture-info-cross-monitor"),
        Some(Reason::DisplayUnknown) => tr("capture-info-display-unknown"),
        Some(Reason::DisplaySdr) => tr("capture-info-display-sdr"),
        Some(Reason::UnsupportedOs) => tr("capture-info-unsupported-os"),
        Some(Reason::HdrFailed) => tr("capture-info-hdr-failed"),
        Some(Reason::AutoSdr) => tr("capture-info-auto-sdr"),
        None => "—",
    }
}
// Paths/diagnostics are display-only; control characters must not forge line labels. Truncate by Unicode characters, never splitting UTF-8.
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
    let downgraded = if source.session_meta.as_ref().is_some_and(|meta| {
        meta.get("downgradedFromHdr")
            .and_then(serde_json::Value::as_bool)
            .is_none()
    }) {
        tr("capture-info-unrecorded")
    } else if source.downgraded {
        tr("common-yes")
    } else {
        tr("common-no")
    };
    let mut text = message(
        "capture-info-summary",
        &[
            ("original", source.original_label().into()),
            ("downgraded", downgraded.into()),
            ("reason", reason(source.reason).into()),
        ],
    );
    if let Some(path) = &source.original {
        text.push('\n');
        text.push_str(&message(
            "capture-info-path",
            &[("path", single_line(&path.to_string_lossy()).into())],
        ));
    }
    if let Some(display) = source.display {
        let hdr = if !display.configuration_known {
            tr("capture-info-unconfirmed")
        } else if display.hdr_enabled {
            tr("capture-info-enabled")
        } else {
            tr("capture-info-disabled")
        };
        let fallback = if display.white_level_known {
            ""
        } else {
            tr("capture-info-fallback")
        };
        text.push('\n');
        text.push_str(&message(
            "capture-info-display",
            &[
                ("hdr", hdr.into()),
                ("white", format!("{:.1}", display.sdr_white_nits).into()),
                ("fallback", fallback.into()),
                ("minimum", format!("{:.3}", display.minimum_nits).into()),
                ("maximum", format!("{:.1}", display.maximum_nits).into()),
            ],
        ));
    }
    if let Some(diagnostic) = &source.diagnostic {
        text.push('\n');
        text.push_str(&message(
            "capture-info-diagnostic",
            &[("diagnostic", single_line(diagnostic).into())],
        ));
    }
    text
}
#[cfg(test)]
#[path = "capture_info_tests.rs"]
mod tests;

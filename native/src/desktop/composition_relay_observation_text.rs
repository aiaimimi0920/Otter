//! Read-only semantic projection; format sensitive values only on explicit viewing, never log/write them to disk or treat audits as executable commands.
use crate::{
    live_relay_observation::{Confidence, Source, State, Summary},
    live_relay_trigger_audit::Outcome,
    live_relay_viewer_business::Business,
};
use std::fmt::Write;
pub(super) const MAX_TEXT: usize = 128 * 1024;
fn end() -> String {
    format!(
        "\r\n{}\r\n",
        crate::ui_text::text("integration-display-limit")
    )
}

#[derive(Default)]
struct Text {
    value: String,
    units: usize,
    truncated: bool,
}
impl Text {
    fn line(&mut self, value: &str) {
        if self.truncated {
            return;
        }
        let left = MAX_TEXT
            - crate::ui_text::text("integration-display-limit")
                .encode_utf16()
                .count()
            - 4
            - self.units;
        if left < 2 {
            self.truncated = true;
            return;
        }
        let mut used = 0;
        for c in value.chars() {
            if used + c.len_utf16() > left - 2 {
                self.truncated = true;
                break;
            }
            self.value.push(c);
            used += c.len_utf16();
        }
        self.value.push_str("\r\n");
        self.units += used + 2;
    }
}
// Remote names/reasons cannot forge UI fields using NUL, newlines, or bidi controls; original business objects remain unchanged.
fn clean(value: &str, limit: usize) -> String {
    let mut result = String::new();
    let mut chars = value.chars();
    for c in chars.by_ref().take(limit) {
        if c.is_control() || matches!(c, '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}') {
            let _ = write!(result, "\\u{{{:x}}}", c as u32);
        } else {
            result.push(c);
        }
    }
    if chars.next().is_some() {
        result.push_str(crate::ui_text::text("integration-truncated"));
    }
    result
}
fn state(value: State) -> &'static str {
    match value {
        State::Unknown => crate::ui_text::text("integration-unknown"),
        State::Detected => crate::ui_text::text("integration-detected"),
        State::Observing => crate::ui_text::text("integration-observing"),
        State::Stable => crate::ui_text::text("integration-stable"),
        State::Triggered => crate::ui_text::text("integration-triggered"),
        State::Stale => crate::ui_text::text("integration-stale"),
        State::Error => crate::ui_text::text("integration-error"),
    }
}
fn source(value: Source) -> &'static str {
    match value {
        Source::UiAutomation => "UIA",
        Source::AppAdapter => crate::ui_text::text("integration-application-adapter"),
        Source::Vision => crate::ui_text::text("integration-vision"),
        Source::Unknown => crate::ui_text::text("integration-unknown"),
    }
}
fn confidence(value: Confidence) -> &'static str {
    match value {
        Confidence::Exact => crate::ui_text::text("integration-exact"),
        Confidence::High => crate::ui_text::text("integration-risk-high"),
        Confidence::Medium => crate::ui_text::text("integration-risk-medium"),
        Confidence::Low => crate::ui_text::text("integration-risk-low"),
    }
}
pub(super) fn render(business: Option<&Business>) -> String {
    let Some(b) = business else {
        return format!(
            "{}\r\n",
            crate::ui_text::text("integration-await-observation")
        );
    };
    let mut text = Text::default();
    let summary = match b.capabilities().summarize(b.observations()) {
        Summary::Unsupported => crate::ui_text::text("integration-unsupported"),
        Summary::Error(_) => crate::ui_text::text("integration-error"),
        Summary::Observing => crate::ui_text::text("integration-observing"),
        Summary::Stable => crate::ui_text::text("integration-stable"),
        Summary::Stale => crate::ui_text::text("integration-stale"),
    };
    text.line(&crate::ui_text::format(
        "integration-observation-summary",
        &[
            ("summary", summary.into()),
            ("count", b.observations().iter().count().into()),
            ("registrations", b.registrations().len().into()),
        ],
    ));
    text.line(crate::ui_text::text("integration-observations-heading"));
    for o in b.observations().iter() {
        if text.truncated {
            break;
        }
        text.line(&format!(
            "{} · {} · {} / {} · #{}",
            clean(&o.observation_id, 160),
            state(o.state),
            source(o.source),
            confidence(o.confidence),
            o.sequence
        ));
        text.line(&crate::ui_text::format(
            "integration-observation-timing",
            &[
                ("sample", o.observed_at_ms.into()),
                (
                    "stable",
                    o.stable_since_ms
                        .map(|n| format!("{n} ms"))
                        .unwrap_or_else(|| "—".into())
                        .into(),
                ),
            ],
        ));
        if let Some(locator) = &o.locator {
            text.line(&format!(
                "  {} · {} · {}",
                clean(&locator.control_type, 160),
                clean(locator.automation_id.as_deref().unwrap_or("—"), 160),
                clean(locator.name.as_deref().unwrap_or("—"), 160)
            ));
        }
        if let Some(value) = &o.value {
            // Each validated value is at most 64 KiB; display at most 512 characters per value, with a separate whole-window UTF-16 gate.
            text.line(&crate::ui_text::format(
                "integration-observation-value",
                &[("value", clean(&value.to_string(), 512).into())],
            ));
        }
        if let Some(reason) = &o.reason {
            text.line(&crate::ui_text::format(
                "integration-observation-reason",
                &[("reason", clean(reason, 256).into())],
            ));
        }
    }
    text.line(crate::ui_text::text("integration-trigger-registry-heading"));
    for r in b.registrations() {
        if text.truncated {
            break;
        }
        text.line(&format!(
            "{} · {} · {} · revision {}",
            clean(&r.binding.binding_id, 160),
            clean(&r.binding.observation_id, 160),
            if r.binding.enabled {
                crate::ui_text::text("integration-enabled")
            } else {
                crate::ui_text::text("integration-disabled")
            },
            r.condition.revision
        ));
        text.line(&crate::ui_text::format(
            "integration-observation-condition",
            &[
                ("operator", format!("{:?}", r.condition.operator).into()),
                (
                    "operand",
                    clean(&r.condition.operand.to_string(), 256).into(),
                ),
                ("stable", r.condition.stable_for_ms.into()),
                (
                    "confidence",
                    confidence(r.condition.minimum_confidence).into(),
                ),
            ],
        ));
        text.line(&format!(
            "  armed={} · lastMatch={} · pendingOfflineMatch={} · risingEdge={} · rearm={}",
            r.armed,
            r.last_match,
            r.pending_offline_match,
            r.condition.rising_edge,
            r.condition.rearm
        ));
        text.line(&crate::ui_text::format(
            "integration-observation-target",
            &[
                ("instance", clean(&r.target.surface_instance_id, 160).into()),
                (
                    "attachment",
                    clean(&r.target.surface_attachment_id, 160).into(),
                ),
                ("node", clean(&r.target.surface_node_id, 160).into()),
                ("event", clean(&r.target.surface_event, 160).into()),
                ("action", clean(&r.target.surface_action, 160).into()),
            ],
        ));
    }
    let count = b.audits().iter().count();
    text.line(&crate::ui_text::format(
        "integration-observation-audits",
        &[("recent", count.min(16).into()), ("count", count.into())],
    ));
    for a in b.audits().iter().skip(count.saturating_sub(16)) {
        if text.truncated {
            break;
        }
        let outcome = match a.outcome {
            Outcome::Fired
                if a.action_request_id.is_none()
                    && a.reason.as_deref() == Some("surface_action_dispatch_reserved") =>
            {
                crate::ui_text::text("integration-dispatching")
            }
            Outcome::Fired => crate::ui_text::text("integration-triggered"),
            Outcome::Skipped => crate::ui_text::text("integration-skipped"),
            Outcome::Failed => crate::ui_text::text("integration-failed"),
        };
        text.line(&format!(
            "{} · {} · {} / #{} · {} ms",
            clean(&a.binding_id, 160),
            outcome,
            clean(&a.observation_id, 160),
            a.observation_sequence,
            a.evaluated_at_ms
        ));
        text.line(&format!(
            "  {} / {} · {}",
            source(a.observation_source),
            clean(&a.authorized_by, 160),
            clean(&a.idempotency_key, 160)
        ));
        if let Some(reason) = &a.reason {
            text.line(&crate::ui_text::format(
                "integration-observation-reason",
                &[("reason", clean(reason, 256).into())],
            ));
        }
        if let Some(request) = &a.action_request_id {
            text.line(&crate::ui_text::format(
                "integration-observation-request",
                &[("request", clean(request, 160).into())],
            ));
        }
    }
    if text.truncated {
        text.value.push_str(&end());
    }
    text.value
}
#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-live")))]
#[path = "composition_relay_observation_text_tests.rs"]
mod tests;

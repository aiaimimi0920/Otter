//! 只读语义投影；敏感值仅在显式查看时格式化，不写日志/磁盘，不把审计当执行命令。
use crate::{
    live_relay_observation::{Confidence, Source, State, Summary},
    live_relay_trigger_audit::Outcome,
    live_relay_viewer_business::Business,
};
use std::fmt::Write;
pub(super) const MAX_TEXT: usize = 128 * 1024;
const END: &str = "\r\n…已达显示上限\r\n";

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
        let left = MAX_TEXT - END.encode_utf16().count() - self.units;
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
// 不允许远端名称/原因以NUL、换行或bidi控制伪造UI字段；原业务对象不变。
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
        result.push_str("…[已截断]");
    }
    result
}
fn state(value: State) -> &'static str {
    match value {
        State::Unknown => "未知",
        State::Detected => "已检测",
        State::Observing => "观察中",
        State::Stable => "稳定",
        State::Triggered => "已触发",
        State::Stale => "过期",
        State::Error => "错误",
    }
}
fn source(value: Source) -> &'static str {
    match value {
        Source::UiAutomation => "UIA",
        Source::AppAdapter => "应用适配器",
        Source::Vision => "视觉",
        Source::Unknown => "未知",
    }
}
fn confidence(value: Confidence) -> &'static str {
    match value {
        Confidence::Exact => "精确",
        Confidence::High => "高",
        Confidence::Medium => "中",
        Confidence::Low => "低",
    }
}
pub(super) fn render(business: Option<&Business>) -> String {
    let Some(b) = business else {
        return "等待已授权观察快照\r\n".into();
    };
    let mut text = Text::default();
    let summary = match b.capabilities().summarize(b.observations()) {
        Summary::Unsupported => "不支持",
        Summary::Error(_) => "错误",
        Summary::Observing => "观察中",
        Summary::Stable => "稳定",
        Summary::Stale => "过期",
    };
    text.line(&format!(
        "观察：{summary} · {} 项 · 触发注册 {} 项",
        b.observations().iter().count(),
        b.registrations().len()
    ));
    text.line("[观察]");
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
        text.line(&format!(
            "  采样 {} ms · 稳定起点 {}",
            o.observed_at_ms,
            o.stable_since_ms
                .map(|n| format!("{n} ms"))
                .unwrap_or_else(|| "—".into())
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
            // 单个已校验值至多64KiB；显示单值最多512字符，总窗口另有UTF-16门。
            text.line(&format!("  值：{}", clean(&value.to_string(), 512)));
        }
        if let Some(reason) = &o.reason {
            text.line(&format!("  原因：{}", clean(reason, 256)));
        }
    }
    text.line("[触发注册 · 最近确认快照]");
    for r in b.registrations() {
        if text.truncated {
            break;
        }
        text.line(&format!(
            "{} · {} · {} · revision {}",
            clean(&r.binding.binding_id, 160),
            clean(&r.binding.observation_id, 160),
            if r.binding.enabled {
                "启用"
            } else {
                "禁用"
            },
            r.condition.revision
        ));
        text.line(&format!(
            "  {:?} {} · 稳定 {} ms · 最低置信度 {}",
            r.condition.operator,
            clean(&r.condition.operand.to_string(), 256),
            r.condition.stable_for_ms,
            confidence(r.condition.minimum_confidence)
        ));
        text.line(&format!(
            "  armed={} · lastMatch={} · pendingOfflineMatch={} · risingEdge={} · rearm={}",
            r.armed,
            r.last_match,
            r.pending_offline_match,
            r.condition.rising_edge,
            r.condition.rearm
        ));
        text.line(&format!(
            "  目标：{} / {} / {} / {} / {}",
            clean(&r.target.surface_instance_id, 160),
            clean(&r.target.surface_attachment_id, 160),
            clean(&r.target.surface_node_id, 160),
            clean(&r.target.surface_event, 160),
            clean(&r.target.surface_action, 160)
        ));
    }
    let count = b.audits().iter().count();
    text.line(&format!("[触发记录 · 最近 {} / {count} 项]", count.min(16)));
    for a in b.audits().iter().skip(count.saturating_sub(16)) {
        if text.truncated {
            break;
        }
        let outcome = match a.outcome {
            Outcome::Fired
                if a.action_request_id.is_none()
                    && a.reason.as_deref() == Some("surface_action_dispatch_reserved") =>
            {
                "派发中"
            }
            Outcome::Fired => "已触发",
            Outcome::Skipped => "已跳过",
            Outcome::Failed => "失败",
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
            text.line(&format!("  原因：{}", clean(reason, 256)));
        }
        if let Some(request) = &a.action_request_id {
            text.line(&format!("  请求：{}", clean(request, 160)));
        }
    }
    if text.truncated {
        text.value.push_str(END);
    }
    text.value
}
#[cfg(test)]
#[path = "composition_relay_observation_text_tests.rs"]
mod tests;

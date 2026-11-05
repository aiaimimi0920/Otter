//! 原32槽队列的Slider尾合并/依赖取消；不合并commit、click或已开始的HTTP，不拥有另一worker。
use crate::tile_surface_actions::{Command, Kind, Queue};
use std::{collections::VecDeque, sync::Arc};
fn same(a: &Command, b: &Command) -> bool {
    a.edit
        .as_ref()
        .zip(b.edit.as_ref())
        .is_some_and(|(a, b)| Arc::ptr_eq(a, b))
}
pub(crate) fn replace_tail(pending: &VecDeque<Command>, commands: &[Command]) -> bool {
    let [next] = commands else {
        return false;
    };
    pending.back().is_some_and(|last| {
        same(last, next)
            && matches!((&last.kind,&next.kind),(Kind::Event(a),Kind::Event(b))
            if a["event"] == "input" && b["event"] == "input")
    })
}
impl Queue {
    /// UI草稿只观察原队列/受理槽，不持另一份完成队列或将accepted误当业务结果。
    pub(crate) fn edit_event_pending(
        &self,
        id: &str,
        edit: &Arc<crate::tile_surface_edit::Edit>,
    ) -> bool {
        edit.in_flight()
            || self
                .accepted
                .get(id)
                .is_some_and(|a| a.edit.as_ref().is_some_and(|e| Arc::ptr_eq(e, edit)))
            || self.pending.iter().any(|c| {
                c.edit.as_ref().is_some_and(|e| Arc::ptr_eq(e, edit))
                    && matches!(&c.kind, Kind::Event(e) if e["eventId"] == id)
            })
    }
    /// 只暂停同Edit前置请求拥有的派发时钟；没有在途/受理身份的普通事件仍按原1500ms过期。
    pub(super) fn waiting_edit(&self, command: &Command, now: std::time::Instant) -> bool {
        command.edit.as_ref().is_some_and(|edit| {
            edit.active()
                && (edit.in_flight_at(now)
                    || self
                        .inflight
                        .as_ref()
                        .is_some_and(|(view, generation, prior)| {
                            view == &command.anchor.view
                                && *generation == command.anchor.generation
                                && prior.upgrade().is_some_and(|e| e.in_flight_at(now))
                        })
                    || self.accepted.values().any(|a| {
                        now < a.until
                            && a.anchor.view == command.anchor.view
                            && a.anchor.generation == command.anchor.generation
                            && a.edit.is_some()
                    }))
        })
    }
    /// 仅新鲜state确认前缀终态时授一次原1500ms派发窗口，轮询/重复GET不续期。
    pub(super) fn release_ready_edits(
        &mut self,
        state: &crate::tile_surface_wire::State,
        now: std::time::Instant,
    ) {
        let ready = self
            .accepted
            .iter()
            .filter(|(event, a)| {
                a.anchor.view == state.view
                    && a.anchor.generation == state.generation
                    && !state
                        .activity
                        .pending
                        .iter()
                        .any(|p| p.event == **event && p.ack.request_id() == a.request)
                    && !state
                        .activity
                        .confirmations
                        .iter()
                        .any(|c| c.event_id == **event && c.request_id == a.request)
            })
            .filter_map(|(_, a)| a.edit.clone())
            .find(|e| e.active())
            .is_some();
        for command in &mut self.pending {
            if command.edit.as_ref().is_some_and(|edit| {
                ready
                    && edit.active()
                    && command.anchor.view == state.view
                    && command.anchor.generation == state.generation
            }) {
                command.until = now + std::time::Duration::from_millis(1500);
            }
        }
    }
    /// 单个UI预览可跨抬起保留；已派发动作仍由原worker拥有，不建立第二队列或延长意图期限。
    pub(crate) fn preview_pending(
        &self,
        edit: &Arc<crate::tile_surface_edit::Edit>,
        frame: &crate::tile_shared::Frame,
        instance: &str,
    ) -> bool {
        edit.active()
            && (edit.in_flight()
                || self
                    .pending
                    .iter()
                    .any(|c| c.edit.as_ref().is_some_and(|e| Arc::ptr_eq(e, edit)))
                || self
                    .accepted
                    .values()
                    .any(|a| a.edit.as_ref().is_some_and(|e| Arc::ptr_eq(e, edit)))
                || self
                    .states
                    .get(instance)
                    .zip(frame.arts.get(instance))
                    .is_some_and(|(s, a)| s.snapshot.revision > a.snapshot.revision))
    }
    pub(crate) fn cancel_edit(&mut self, edit: &Arc<crate::tile_surface_edit::Edit>) {
        edit.stop();
        self.retire_edit_dispatch(edit);
        self.pending
            .retain(|c| c.edit.as_ref().is_none_or(|e| !e.same_group(edit)));
    }
}
/// 只忽略本手势Slider的权威value；祖先click同样不能忽略任何其它节点/属性变化。
pub(crate) fn same_contracts(
    before: &serde_json::Value,
    after: &serde_json::Value,
    field: impl Fn(&str) -> bool,
) -> bool {
    fn strip(value: &mut serde_json::Value, field: &impl Fn(&str) -> bool) {
        if value["id"].as_str().is_some_and(field) {
            if let Some(props) = value["props"].as_object_mut() {
                props.remove("value");
            }
        }
        if let Some(children) = value["children"].as_array_mut() {
            for child in children {
                strip(child, field);
            }
        }
    }
    let (mut before, mut after) = (before.clone(), after.clone());
    strip(&mut before, &field);
    strip(&mut after, &field);
    before == after
}

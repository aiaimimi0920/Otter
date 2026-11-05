//! 菜单的连线有效参数显示快照；不复制未连线手动值，身份变化使旧菜单失效。
use super::{configuration_stamp::Stamp, State};
use serde_json::{Map, Value};
use std::sync::Arc;
pub(super) struct View {
    pub identity: Arc<()>,
    pub values: Map<String, Value>,
    pub linked: Vec<String>,
    configuration: Stamp,
}
impl View {
    pub fn update(state: &State, values: Map<String, Value>, linked: Vec<String>) {
        let mut view = state.parameter_view.borrow_mut();
        if view.as_ref().is_some_and(|old| {
            old.configuration.matches(state) && old.values == values && old.linked == linked
        }) {
            return;
        }
        *view = Some(Self {
            identity: Arc::new(()),
            values,
            linked,
            configuration: Stamp::capture(state),
        });
    }
    pub fn current(&self, state: &State) -> bool {
        self.configuration.matches(state)
    }
    /// 同步本地未连线文本提交不会改变拓扑或上游值；只推进仍属原配置的视图。
    pub fn after_local_text(state: &State, before: &Stamp) {
        let after = Stamp::capture(state);
        if let Some(view) = state.parameter_view.borrow_mut().as_mut() {
            if view.configuration == *before {
                view.configuration = after;
            }
        }
    }
}
pub(super) fn matches(identity: &Option<Arc<()>>, state: &State) -> bool {
    match (identity, state.parameter_view.borrow().as_ref()) {
        (None, None) => true,
        (Some(identity), Some(view)) => {
            Arc::ptr_eq(identity, &view.identity) && view.current(state)
        }
        _ => false,
    }
}

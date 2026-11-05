//! 应用拥有已注册请求的配置凭证；只结束匹配代际，远端取消和租约释放归任务集合。
use super::{configuration_stamp::Stamp, State};
use crate::graph_requests::{Request, Requests};
pub(super) struct Binding {
    pub request: Request,
    pub stamp: Stamp,
    pub topology: crate::workspace::TopologyStamp,
}
pub(super) fn retire_changed(
    state: &State,
    registry: &mut Requests,
    workspace: &crate::workspace::Workspace,
) {
    let stale = state
        .execution_configuration
        .borrow()
        .as_ref()
        .is_some_and(|binding| {
            !binding.stamp.matches(state) || !binding.topology.matches(workspace)
        });
    if !stale {
        return;
    }
    let binding = state
        .execution_configuration
        .borrow_mut()
        .take()
        .expect("checked binding");
    // 旧绑定不得结束已经替换的注册请求，或覆盖其状态。
    if !registry.finish(&state.unit_id, &binding.request) {
        return;
    }
    if state
        .pending_execution
        .borrow()
        .as_ref()
        .is_some_and(|pending| pending.request == binding.request)
    {
        state.pending_execution.borrow_mut().take();
    }
    state
        .execution_results
        .borrow_mut()
        .cancel(&binding.request);
}
pub(super) fn matches(
    state: &State,
    request: &Request,
    workspace: &crate::workspace::Workspace,
) -> bool {
    match state.execution_configuration.borrow().as_ref() {
        Some(binding) => {
            binding.request == *request
                && binding.stamp.matches(state)
                && binding.topology.matches(workspace)
        }
        // 独立图/模型入口可没有 UI 选择；已选择能力的文档不能绕过凭证。
        None => state.selected_art.borrow().is_none(),
    }
}
pub(super) fn clear(state: &State, request: &Request) {
    let matched = state
        .execution_configuration
        .borrow()
        .as_ref()
        .is_some_and(|binding| binding.request == *request);
    if matched {
        state.execution_configuration.borrow_mut().take();
    }
}
#[cfg(test)]
#[path = "execution_configuration_tests.rs"]
mod tests;

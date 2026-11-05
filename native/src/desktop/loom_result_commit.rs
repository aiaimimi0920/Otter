//! 完成报告到节点输出的同步事务边界；不运行消息循环或网络，不丢弃清理 owner。
use super::{Completion, Report};
use crate::{
    desktop::{loom_delivery::TakeError, loom_result_state::Results},
    editor::Editor,
    graph_requests::Requests,
    workspace_id::UnitId,
};

#[derive(Debug, PartialEq)]
pub enum CommitError {
    Stale,
    ExecutionFailed,
    PendingCleanup,
    Output(TakeError),
    State(crate::desktop::loom_result_state::Error),
}
impl Report {
    /// UI 判定失败后的收尾；只结束仍匹配的请求，completion 仍保留待清理资源。
    /// 不要求旧文档仍有效：文档变化本身也可能是拒绝提交的原因。
    pub fn reject(&mut self, registry: &mut Requests, state: &mut Results, message: &str) -> bool {
        if state.unit() != &self.unit
            || !registry.is_latest(&self.unit, &self.request)
            || state.active() != Some(&self.request)
        {
            return false;
        }
        if !state.fail(&self.request, message) {
            return false;
        }
        let finished = registry.finish(&self.unit, &self.request);
        debug_assert!(finished);
        self.current = false;
        true
    }
    /// 正式端口提交成功后转移候选元数据，候选 UI 的选择与缓存合并由后续消费者负责。
    /// lookup 必须只读且不重入 UI；调用后不能复用此报告提交另一节点。
    pub fn commit<'a>(
        &mut self,
        registry: &mut Requests,
        state: &mut Results,
        lookup: impl FnMut(&UnitId) -> Option<&'a Editor>,
    ) -> Result<(), CommitError> {
        let mut lookup = lookup;
        self.commit_checked(registry, state, |unit, stamp| {
            lookup(unit).is_some_and(|editor| stamp.matches(editor) && editor.draft().is_none())
        })
    }
    pub(crate) fn commit_checked(
        &mut self,
        registry: &mut Requests,
        state: &mut Results,
        check: impl FnMut(&UnitId, &crate::desktop::loom_input_stamp::SourceStamp) -> bool,
    ) -> Result<(), CommitError> {
        if state.unit() != &self.unit
            || state.active() != Some(&self.request)
            || !self.current
            || !registry.is_latest(&self.unit, &self.request)
            || !self.freshness.is_current(check)
        {
            return Err(CommitError::Stale);
        }
        match &mut self.completion {
            Completion::Shader(completion) => {
                let completion = completion
                    .as_mut()
                    .map_err(|_| CommitError::ExecutionFailed)?;
                let batch = completion
                    .take(&self.unit, &self.request)
                    .ok_or(CommitError::Stale)?;
                state
                    .finish(&self.request, batch)
                    .map_err(CommitError::State)?;
            }
            Completion::Remote(completion) => {
                let completion = completion
                    .as_mut()
                    .map_err(|_| CommitError::ExecutionFailed)?;
                if completion.cleanup_pending() {
                    return Err(CommitError::PendingCleanup);
                }
                let result = completion
                    .result
                    .as_mut()
                    .map_err(|_| CommitError::ExecutionFailed)?;
                // 独立核对报告和底层交付身份，避免错误组装的公开报告跨节点提交。
                if completion.unit != self.unit
                    || completion.request != self.request
                    || result.delivery.unit() != &self.unit
                    || result.delivery.request() != &self.request
                {
                    return Err(CommitError::Stale);
                }
                let batch = result
                    .delivery
                    .take_released()
                    .map_err(CommitError::Output)?;
                state
                    .finish(&self.request, batch)
                    .map_err(CommitError::State)?;
                state.receive_candidates(result.candidates.take());
            }
        }
        // 上方没有外部回调/await，不存在核验后注册表被替换的调度间隙。
        let finished = registry.finish(&self.unit, &self.request);
        debug_assert!(finished);
        self.current = false;
        Ok(())
    }
}

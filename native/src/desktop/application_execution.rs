//! 应用线程交付已完成的节点结果；短借用模型，不联网、不运行嵌套消息循环。
use super::Application;
#[cfg(test)]
#[path = "application_execution_tests.rs"]
mod tests;
impl Application {
    pub(super) fn cancel_requested_executions(&mut self) {
        for document in &self.documents {
            super::super::execution_configuration::retire_changed(
                &document.state,
                &mut self.executions,
                &self.workspace,
            );
            let Some(cancel) = document.state.execution_cancel.borrow_mut().take() else {
                continue;
            };
            if let Some(ticket) = cancel.preparation {
                let current = document
                    .state
                    .pending_preparation
                    .borrow()
                    .as_ref()
                    .is_some_and(|pending| pending.ticket == ticket);
                if current {
                    document.state.pending_preparation.borrow_mut().take();
                    *document.state.preparation_error.borrow_mut() = None;
                }
                self.input_jobs.cancel(&ticket);
            }
            if let Some(request) = cancel.execution {
                if self.executions.finish(&document.state.unit_id, &request) {
                    super::super::execution_configuration::clear(&document.state, &request);
                    if document
                        .state
                        .pending_execution
                        .borrow()
                        .as_ref()
                        .is_some_and(|execution| execution.request == request)
                    {
                        document.state.pending_execution.borrow_mut().take();
                    }
                    document
                        .state
                        .execution_results
                        .borrow_mut()
                        .cancel(&request);
                }
            }
        }
        self.execution_jobs.retire_stale(&self.executions);
    }
    pub(super) fn start_pending_executions(&mut self) {
        for document in &self.documents {
            // 清理中的 worker 同样占槽，不能把暂时繁忙当成执行失败或丢弃已编码输入。
            if self.execution_jobs.len() >= 2 {
                break;
            }
            let Some(mut execution) = document.state.pending_execution.borrow_mut().take() else {
                continue;
            };
            let unit = execution.unit.clone();
            let request = execution.request.clone();
            // 来源窗口不能借用另一节点的请求；过期排队请求不能结束替换后的请求。
            if unit != document.state.unit_id || !self.executions.is_latest(&unit, &request) {
                continue;
            }
            let current = execution.inputs.freshness().is_ok_and(|freshness| {
                freshness.is_current(|unit, stamp| {
                    stamp.matches_owners(
                        unit,
                        &self.documents,
                        &self.live_sessions,
                        &self.workspace,
                    )
                })
            });
            let mut results = document.state.execution_results.borrow_mut();
            results.begin(request.clone());
            if !current
                || !super::super::execution_configuration::matches(
                    &document.state,
                    &request,
                    &self.workspace,
                )
            {
                results.fail(&request, "准备期间文档已变化，执行未启动");
                self.executions.finish(&unit, &request);
                super::super::execution_configuration::clear(&document.state, &request);
                continue;
            }
            // 已替换注册令牌时先取消旧 worker，仍在清理的 owner 继续占槽。
            self.execution_jobs.retire_stale(&self.executions);
            self.shader_cache.bind(&document.state, &mut execution);
            if self
                .execution_jobs
                .start(execution, &self.executions)
                .is_err()
            {
                results.fail(&request, "无法启动节点执行，后台任务可能仍在清理");
                self.executions.finish(&unit, &request);
                super::super::execution_configuration::clear(&document.state, &request);
            }
        }
    }
    pub(super) fn collect_execution_updates(&self) {
        for document in &self.documents {
            let Some(updates) = self.execution_jobs.take_updates_checked(
                &document.state.unit_id,
                &self.executions,
                |unit, stamp| {
                    stamp.matches_owners(
                        unit,
                        &self.documents,
                        &self.live_sessions,
                        &self.workspace,
                    )
                },
            ) else {
                continue;
            };
            let mut results = document.state.execution_results.borrow_mut();
            if let Some(progress) = updates.progress {
                results.set_progress(&updates.request, progress);
            }
            if let Some(crate::desktop::loom_execution_session::Update::Preview {
                revision,
                mut delivery,
            }) = updates.preview
            {
                // Session 只发布已释放租约的预览；这里仍核对交付身份和领取条件。
                if delivery.unit() == &document.state.unit_id
                    && delivery.request() == &updates.request
                {
                    if let Ok(batch) = delivery.take_released() {
                        let _ = results.preview(&updates.request, revision, batch);
                    }
                }
            }
        }
    }
    pub(super) fn collect_execution_results(&mut self) {
        while let Some(mut report) = self.execution_jobs.take_settled(&self.executions) {
            let Some(document) = self
                .documents
                .iter()
                .find(|document| document.state.unit_id == report.unit)
            else {
                self.executions.finish(&report.unit, &report.request);
                continue;
            };
            // 自引用输出也可能是依赖，先做短只读检查，再借用目标结果；中间不得重入 UI。
            let current = report.dependencies_current(|unit, stamp| {
                stamp.matches_owners(unit, &self.documents, &self.live_sessions, &self.workspace)
            });
            let mut results = document.state.execution_results.borrow_mut();
            let result = report.commit_checked(&mut self.executions, &mut results, |_, _| current);
            let committed = result.is_ok();
            if committed {
                results.bind_candidate_configuration(
                    super::super::configuration_stamp::Stamp::capture(&document.state),
                );
            }
            if let Err(error) = result {
                // 不把服务原始响应/认证信息放进 UI 状态；详细原因由类型区分。
                let message = match error {
                    crate::desktop::loom_execution_tasks::CommitError::Stale => {
                        "执行期间文档或请求已变化，结果未应用"
                    }
                    crate::desktop::loom_execution_tasks::CommitError::ExecutionFailed => {
                        "节点执行失败"
                    }
                    crate::desktop::loom_execution_tasks::CommitError::PendingCleanup => {
                        "远端资源清理尚未确认"
                    }
                    crate::desktop::loom_execution_tasks::CommitError::Output(_) => {
                        "无法读取节点执行结果"
                    }
                    crate::desktop::loom_execution_tasks::CommitError::State(_) => {
                        "节点输出超过资源限制，结果未应用"
                    }
                };
                report.reject(&mut self.executions, &mut results, message);
            }
            super::super::execution_configuration::clear(&document.state, &report.request);
            drop(results);
            if committed {
                self.shader_cache.record(&document.state, &mut report);
                // 结果事务结束并释放借用后再通知；派发仍由主循环统一准入。
                unsafe {
                    self.propagate_result(&report.unit);
                }
            }
        }
    }
}

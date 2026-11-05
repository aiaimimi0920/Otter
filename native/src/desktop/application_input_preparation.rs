//! 准备 ticket 与不可变配置配对后才注册执行请求，不重新读取可变 UI 参数。
use super::Application;
impl Application {
    pub(super) fn retire_preparations(&mut self) {
        for document in &self.documents {
            super::super::configuration_stamp::discard_changed(&document.state, &self.workspace);
        }
        self.input_jobs.retain_requests(|unit, ticket| {
            self.documents
                .iter()
                .find(|document| document.state.unit_id == *unit)
                .is_some_and(|document| {
                    document
                        .state
                        .pending_preparation
                        .borrow()
                        .as_ref()
                        .is_some_and(|pending| {
                            pending.ticket == *ticket
                                && pending.selection.matches(&document.state)
                                && pending.topology.matches(&self.workspace)
                        })
                })
        });
        self.input_jobs.discard_retired();
    }
    pub(super) fn collect_prepared_inputs(&mut self) {
        while let Some(report) = self.input_jobs.take_matching(|unit, ticket| {
            self.documents
                .iter()
                .find(|document| document.state.unit_id == *unit)
                .is_some_and(|document| {
                    document
                        .state
                        .pending_preparation
                        .borrow()
                        .as_ref()
                        .is_some_and(|pending| {
                            pending.ticket == *ticket
                                && pending.selection.matches(&document.state)
                                && pending.topology.matches(&self.workspace)
                        })
                })
        }) {
            let document = self
                .documents
                .iter()
                .find(|document| document.state.unit_id == report.target)
                .expect("matched document");
            let pending = document
                .state
                .pending_preparation
                .borrow_mut()
                .take()
                .expect("matched ticket");
            let result = report
                .result
                .map_err(|_| "图片输入准备失败或已取消")
                .and_then(|mut inputs| {
                    if inputs.target != document.state.unit_id
                        || document.state.pending_execution.borrow().is_some()
                    {
                        return Err("节点已有待执行请求或准备结果身份不匹配");
                    }
                    inputs
                        .set_dependencies(pending.dependencies)
                        .map_err(|_| "执行依赖超过预算")?;
                    pending
                        .plan
                        .stage_checked(&mut self.executions, inputs, |unit, stamp| {
                            stamp.matches_owners(
                                unit,
                                &self.documents,
                                &self.live_sessions,
                                &self.workspace,
                            )
                        })
                        .map_err(|_| "执行配置或文档已失效，未创建新请求")
                });
            match result {
                Ok(mut execution) => {
                    execution.shader = pending.shader;
                    *document.state.execution_configuration.borrow_mut() =
                        Some(super::super::execution_configuration::Binding {
                            request: execution.request.clone(),
                            stamp: pending.selection,
                            topology: pending.topology,
                        });
                    *document.state.preparation_error.borrow_mut() = None;
                    *document.state.pending_execution.borrow_mut() = Some(execution);
                }
                Err(message) => {
                    *document.state.preparation_error.borrow_mut() = Some(message.into())
                }
            }
        }
    }
}

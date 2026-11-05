//! 文件参数完成后在 UI owner 验证配置/拓扑再提交；单 worker，无后台排队线程或空闲定时器。
use super::*;
#[cfg(test)]
#[path = "application_sticker_input_tests.rs"]
mod sticker_tests;
impl Application {
    pub(super) unsafe fn pending_parameter_files(&mut self, report: bool) -> Result<()> {
        if self.modals.blocked() {
            return Ok(());
        }
        if let Some(job) = &self.parameter_file_job {
            if !job.topology.matches(&self.workspace)
                || !self.documents.iter().any(|doc| {
                    doc.state.unit_id == job.unit
                        && job.request.matches(&doc.state)
                        && doc.state.parameter_file_request.borrow().is_none()
                })
            {
                job.cancel();
            }
        }
        let completed = self.parameter_file_job.as_mut().and_then(|job| job.poll());
        if let Some(result) = completed {
            let job = self.parameter_file_job.take().unwrap();
            if !job.cancelled() && job.topology.matches(&self.workspace) {
                if let Some(doc) = self
                    .documents
                    .iter()
                    .find(|doc| doc.state.unit_id == job.unit && job.request.matches(&doc.state))
                {
                    let commit = (|| -> Result<()> {
                        let value = result.map_err(|error| Error::new(E_FAIL, error))?;
                        let (id, value) = match (&job.request.target, value) {
                            (
                                super::super::parameter_file_input::Target::Sticker,
                                super::super::parameter_file_job::Loaded::Sticker(asset),
                            ) => {
                                return doc.state.sticker_input.set_asset(Some(asset));
                            }
                            (
                                super::super::parameter_file_input::Target::Parameter(id),
                                super::super::parameter_file_job::Loaded::Parameter(value),
                            ) => (id, value),
                            _ => return Err(Error::new(E_ABORT, "文件导入目标已失效")),
                        };
                        let mut candidate = doc
                            .state
                            .selected_art
                            .borrow()
                            .as_ref()
                            .ok_or_else(|| Error::new(E_FAIL, "能力已关闭"))?
                            .parameters
                            .clone();
                        let changed = candidate
                            .set(id, Some(serde_json::Value::String(value)))
                            .map_err(|_| {
                                Error::new(E_INVALIDARG, "文件超过参数总容量限制或参数已失效")
                            })?;
                        if changed {
                            super::super::execution_cancel::request(doc.window.0, &doc.state)?;
                            doc.state
                                .selected_art
                                .borrow_mut()
                                .as_mut()
                                .unwrap()
                                .parameters = candidate;
                            super::super::execution_start::parameter_changed(&doc.state);
                        }
                        Ok(())
                    })();
                    if let Err(error) = commit {
                        if !report {
                            return Err(error);
                        }
                        MessageBoxW(
                            Some(doc.window.0),
                            &HSTRING::from(error.to_string()),
                            w!("Otter"),
                            MB_OK | MB_ICONWARNING,
                        );
                    }
                }
            }
        }
        if self.parameter_file_job.is_none() {
            for doc in &self.documents {
                let Some(request) = doc.state.parameter_file_request.borrow_mut().take() else {
                    continue;
                };
                if !request.matches(&doc.state) {
                    continue;
                }
                self.parameter_file_job = Some(
                    super::super::parameter_file_job::Job::start(
                        doc.state.unit_id.clone(),
                        request,
                        self.workspace.topology_stamp(),
                        self.execution_wake.callback(),
                    )
                    .map_err(|error| Error::new(E_FAIL, error.to_string()))?,
                );
                break;
            }
        }
        // UI动作和后台导入均由同一owner推进配置，再沿既有通知链唤醒下游。
        for doc in &self.documents {
            if doc.state.sticker_input.take_changed() {
                self.propagate_created_child(&doc.state.unit_id);
            }
        }
        Ok(())
    }
}

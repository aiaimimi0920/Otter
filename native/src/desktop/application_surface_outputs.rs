//! 在当前获授权窗口与 Art 节点间交付正式输出；不删除收据、不下载描述符资源。
use super::*;
impl Application {
    pub(in crate::desktop::application) fn apply_surface_outputs(&mut self, now: u64) {
        let origin = {
            let view = self.authorization.borrow();
            if view.closed || view.busy || view.request.is_some() || self.exit_requested.get() {
                return;
            }
            view.origin.clone()
        };
        // 附件 owner 的一次性注册先于结果消费；普通交付不再自行首次绑定/换绑。
        for window in self.surface_windows.iter_mut() {
            if !window.result_binding_pending()
                || origin.as_deref() != Some(window.binding().origin())
            {
                continue;
            }
            let result = window.bind_node_results(now, |binding| {
                let view = self.authorization.borrow();
                if view.closed
                    || view.busy
                    || view.request.is_some()
                    || view.origin != origin
                    || self.exit_requested.get()
                {
                    return Ok(false);
                }
                drop(view);
                let Some(document) = self
                    .documents
                    .iter()
                    .find(|document| document.state.unit_id.as_str() == binding.node())
                else {
                    return Ok(false);
                };
                if !document.state.is_art() {
                    return Ok(false);
                }
                let mut results = document.state.execution_results.borrow_mut();
                results.bind_surface(binding).map_err(|error| {
                    windows::core::Error::new(
                        windows::Win32::Foundation::E_FAIL,
                        format!("Surface result binding rejected: {error:?}"),
                    )
                })?;
                Ok(true)
            });
            if let Err(error) = result {
                self.authorization.borrow_mut().status = format!("Surface 节点绑定未完成：{error}");
            }
        }
        // 无新结果时不分配通知缓存；首次写入前一次预留每节点最多一条通知。
        let mut changed = Vec::new();
        self.remote_sessions
            .visit_surface_outputs(|binding, commit, revision| {
                if origin.as_deref() != Some(binding.origin()) {
                    return;
                }
                let Some(window) = self
                    .surface_windows
                    .iter_mut()
                    .find(|window| window.binding() == binding)
                else {
                    return;
                };
                if window.refresh(now).is_err()
                    || !window.is_shown()
                    || window.stopped().is_some()
                    || window.snapshot_revision() != revision
                    || !window
                        .lifecycle()
                        .is_some_and(crate::surface_lifecycle_owner::Owner::confirmed_active)
                {
                    return;
                }
                // refresh 之后再借用状态，防止窗口关闭同步重入时发生 RefCell 冲突。
                let view = self.authorization.borrow();
                if view.closed
                    || view.busy
                    || view.request.is_some()
                    || view.origin != origin
                    || self.exit_requested.get()
                {
                    return;
                }
                drop(view);
                let Some(document) = self
                    .documents
                    .iter()
                    .find(|document| document.state.unit_id.as_str() == binding.node())
                else {
                    return;
                };
                if !document.state.is_art() {
                    return;
                }
                let mut results = document.state.execution_results.borrow_mut();
                if results.has_surface_result(binding, commit.revision()) {
                    return;
                }
                if changed.capacity() == 0 && changed.try_reserve(self.documents.len()).is_err() {
                    self.authorization.borrow_mut().status = "无法准备 Surface 输出通知".into();
                    return;
                }
                match results.apply_surface_result(binding, commit) {
                    Ok(true) => {
                        if !changed.contains(&&document.state.unit_id) {
                            changed.push(&document.state.unit_id);
                        }
                    }
                    Ok(false) => {}
                    Err(error) => {
                        self.authorization.borrow_mut().status =
                            format!("Surface 输出未应用：{error:?}")
                    }
                }
            });
        // 所有结果借用已释放，再调用既有传播入口；执行仍由主循环统一准入。
        for unit in changed {
            unsafe {
                self.propagate_result(unit);
            }
        }
    }
}

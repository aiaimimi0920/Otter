//! 下载与图像准备分阶段接纳；原token/宿主/来源全部有效才允许创建签名child。
use super::*;
impl Owner {
    pub(super) fn prepare_windows(
        &mut self,
        documents: &[DocumentWindow],
        workspace: &mut Workspace,
        sessions: &[LiveSession],
    ) -> Option<String> {
        let mut status = None;
        while let Some(report) = self.tasks.take_settled() {
            let Some(entry) = self
                .entries
                .iter_mut()
                .find(|entry| entry.active && entry.unit == report.unit)
            else {
                continue;
            };
            let result = (|| {
                let prepared = report.result.map_err(|_| "Art 描述或准备失败")?;
                let binding = prepared.binding();
                if !current(entry, documents, workspace, sessions)
                    || binding.instance_id != entry.instance
                    || binding.request_id != entry.token.id.as_str()
                    || binding.generation != entry.token.generation
                    || Some(binding.workspace_id.as_str()) != self.workspace.as_deref()
                {
                    return Err("Art 宿主已变化，旧准备已丢弃");
                }
                ports::accept(entry, &prepared, documents, workspace)?;
                entry.images.start(
                    prepared,
                    documents,
                    sessions,
                    workspace,
                    &entry.unit,
                    self.notify.clone(),
                )
            })();
            if let Err(message) = result {
                entry.active = false;
                entry.images.cancel();
                self.tasks.cancel(&entry.unit);
                status = Some(message.into());
                self.status_owner = None;
            }
        }
        for entry in &mut self.entries {
            let Some(result) = entry.images.poll() else {
                continue;
            };
            if !entry.active {
                continue;
            }
            let result = (|| {
                let (prepared, content) = result?;
                if !current(entry, documents, workspace, sessions) {
                    return Err("Art 图片来源已变化，旧准备已丢弃");
                }
                // node为本次宿主映射身份，不要求历史UnitId满足网络标识符字符集。
                let view = crate::surface_live_binding::Binding::local_art(
                    prepared.binding(),
                    &entry.instance,
                    &entry.token.id.to_string(),
                )
                .map_err(|_| "无法建立 Art 本地视图")?;
                entry.window = Some(
                    Window::start_with_content(prepared, view, self.notify.clone(), content)
                        .map_err(|_| "无法创建 Art 宿主或启动执行线程")?,
                );
                self.tasks.mark_running(&entry.unit, &entry.token);
                Ok(())
            })();
            if let Err(message) = result {
                entry.active = false;
                entry.images.cancel();
                self.tasks.cancel(&entry.unit);
                status = Some(message.into());
                self.status_owner = None;
            }
        }
        status
    }
}

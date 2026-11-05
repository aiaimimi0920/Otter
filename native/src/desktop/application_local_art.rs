//! 用户明确的 HTTPS OpenArt 意图进入本地链；消费时重新检查发起节点，绝不从旧 WSS 目录猜 origin。
use super::Application;
use crate::device_authorization_config::Config;
impl Application {
    pub(super) fn start_local_art(
        &mut self,
        origin: &str,
        art: String,
        node: &str,
        config: Config,
    ) -> Result<(), String> {
        if self.modals.blocked() || self.exit_requested.get() || self.restore_job.is_some() {
            return Err("工作区正在切换或退出，Art 未启动".into());
        }
        if config.origin() != origin {
            return Err("Art 服务配置不匹配".into());
        }
        let document = self
            .documents
            .iter()
            .find(|document| {
                document.state.unit_id.as_str() == node
                    // Application 在所属 UI 线程持有 DocumentWindow，检查同时比较 userdata 身份。
                    && unsafe { document.is_alive() }
                    && self.workspace.contains(&document.state.unit_id)
            })
            .ok_or("Art 发起节点已关闭或被替换")?;
        let now = crate::desktop::surface_windows::now_ms().map_err(|_| "无法读取授权时钟")?;
        // 只允许新owner中无在线会话的origin尝试离线；拒绝/容量不足不降级。
        let lease = match self.authorization_owner.lease(origin, now) {
            Ok(lease) => lease,
            Err(crate::device_authorization_owner::Error::Credential)
                if self.authorization_owner.can_resume_local(origin) =>
            {
                self.art_issuers
                    .pending_local(origin)
                    .map_err(|_| "请先获取有效设备授权")?
            }
            Err(_) => return Err("请先获取有效设备授权".into()),
        };
        self.local_arts
            .start(
                document,
                &self.workspace,
                art,
                std::sync::Arc::new(lease),
                &mut self.art_issuers,
                config,
            )
            .map_err(str::to_owned)
    }
    pub(super) fn process_local_arts(&mut self, visible: bool) {
        self.local_arts
            .accept_triggers(self.remote_sessions.take_local_triggers());
        let mut status = self.local_arts.poll(
            &self.documents,
            &mut self.workspace,
            &self.live_sessions,
            visible
                && !self.modals.blocked()
                && self.restore_job.is_none()
                && !self.exit_requested.get(),
        );
        if !self.modals.blocked() && self.restore_job.is_none() && !self.exit_requested.get() {
            let (changed, error) = self.local_arts.apply_results(
                &self.documents,
                &self.workspace,
                &self.live_sessions,
            );
            if error.is_some() {
                status = error;
            }
            for unit in changed {
                unsafe {
                    self.propagate_result(&unit);
                }
            }
        }
        let mut view = self.authorization.borrow_mut();
        view.mount_busy = self.remote_sessions.attaching() || self.local_arts.busy();
        if let Some(status) = status {
            view.status = status;
        }
    }
}

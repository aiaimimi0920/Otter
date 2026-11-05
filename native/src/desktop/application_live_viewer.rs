//! Application观看准入：真实Surface或在线本地Art独立证明；共享目录/新Lease和取消围栏。
use super::*;
use crate::{
    desktop::composition::relay::sessions::ViewerHost,
    desktop::device_authorization_ui::{live, Request},
    device_authorization_config::Config,
};
impl Application {
    fn live_viewer_bindings(&mut self) -> Result<Vec<ViewerHost>> {
        let now = crate::desktop::surface_windows::now_ms()?;
        let mut bindings = Vec::new();
        for window in self.surface_windows.iter_mut() {
            window.refresh(now)?;
            if window.is_shown()
                && window
                    .lifecycle()
                    .is_some_and(|owner| owner.confirmed_active())
            {
                bindings.push(ViewerHost::Surface(window.binding().clone()));
            }
        }
        bindings.extend(
            self.local_arts
                .viewer_proofs(&self.documents, &self.workspace, &self.live_sessions)
                .into_iter()
                .map(ViewerHost::LocalArt),
        );
        Ok(bindings)
    }
    /// 在领取Ready之前结算发起附件；不将已打开观看绑定到面板/工作区寿命。
    pub(super) fn settle_live_viewers(&mut self) -> Result<()> {
        let bindings = self.live_viewer_bindings()?;
        self.remote_sessions.cancel_pending_viewers(|expected| {
            bindings.iter().any(|binding| binding.same_host(expected))
        });
        let watching = self.remote_sessions.viewers();
        let mut view = self.authorization.borrow_mut();
        view.live.bindings = if view.closed {
            Vec::new()
        } else {
            bindings
                .into_iter()
                .filter(|binding| view.origin.as_deref() == Some(binding.origin()))
                .collect()
        };
        view.live.watching = watching;
        Ok(())
    }
    pub(super) fn live_viewer_requests(
        &mut self,
        config: &mut impl FnMut(
            &str,
        ) -> std::result::Result<
            Config,
            crate::device_authorization_config::Error,
        >,
    ) {
        let request = {
            let mut view = self.authorization.borrow_mut();
            if matches!(
                view.request,
                Some(Request::WatchLive(_) | Request::StopLive(_))
            ) {
                view.request.take()
            } else {
                None
            }
        };
        let Some(request) = request else {
            return;
        };
        let result = match request {
            Request::StopLive(key) => {
                if self.remote_sessions.stop_viewer(&key) {
                    Ok("正在停止远程观看…")
                } else {
                    Err("观看会话已结束".into())
                }
            }
            Request::WatchLive(selected) => self
                .start_live_viewer(selected, config)
                .map(|()| "正在加入远程观看…"),
            _ => unreachable!(),
        };
        // stop可能同步销毁HWND；不跨它持有菜单View借用。
        self.authorization.borrow_mut().live.watching = self.remote_sessions.viewers();
        self.authorization.borrow_mut().live.status = match result {
            Ok(status) => status.into(),
            Err(error) => error,
        };
    }
    fn start_live_viewer(
        &mut self,
        selected: live::Selection,
        config: &mut impl FnMut(
            &str,
        ) -> std::result::Result<
            Config,
            crate::device_authorization_config::Error,
        >,
    ) -> std::result::Result<(), String> {
        if self.exit_requested.get() || self.modals.blocked() || self.restore_job.is_some() {
            return Err("应用正在切换或退出，观看未启动".into());
        }
        self.settle_live_viewers()
            .map_err(|error| error.to_string())?;
        if !live::current(&self.authorization.borrow(), &selected) {
            return Err("目录或观看宿主已变化，请重新选择".into());
        }
        if !super::live::fits_wait_budget(
            self.live_sessions.len(),
            self.surface_windows.wait_budget(),
        ) {
            return Err("窗口等待容量已满".into());
        }
        let now = crate::desktop::surface_windows::now_ms().map_err(|error| error.to_string())?;
        let lease = self
            .authorization_owner
            .lease(&selected.origin, now)
            .map_err(|error| format!("请先获取有效授权：{error:?}"))?;
        if lease.device_id() != selected.binding.device() {
            return Err("当前设备与观看宿主不一致".into());
        }
        let config = config(&selected.origin).map_err(|error| format!("配置无效：{error:?}"))?;
        match selected.binding {
            ViewerHost::Surface(binding) => {
                let target = binding
                    .viewer_target(
                        &selected.identity,
                        &selected.origin,
                        lease.device_id(),
                        binding.node(),
                        binding.generation(),
                    )
                    .map_err(|error| format!("观看绑定无效：{error:?}"))?;
                self.remote_sessions
                    .start_viewing(config, lease, target, binding)
            }
            ViewerHost::LocalArt(proof) => self.remote_sessions.start_local_art_viewing(
                config,
                lease,
                crate::live_relay_control_runtime::LocalArtTarget {
                    session: selected.identity.session().into(),
                    selected: selected.identity,
                    proof,
                },
            ),
        }
        .map_err(|error| format!("观看未启动：{error}"))
    }
}

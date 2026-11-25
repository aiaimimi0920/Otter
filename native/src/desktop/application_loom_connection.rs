//! Local Loom integration for the ordinary product; enable only in the message pump, so constructing Application in tests never implicitly networks.
use super::*;
use crate::desktop::device_authorization_ui::Request;
#[path = "application_loom_preference.rs"]
mod preference;
fn profile() -> Option<std::path::PathBuf> {
    let root = std::env::var_os("OTTER_NATIVE_PROFILE_DIR");
    let local = std::env::var_os("LOCALAPPDATA");
    crate::settings_profile::select(None, root.as_deref(), local.as_deref(), false)
        .ok()
        .flatten()?
        .path
        .parent()
        .map(|p| p.to_owned())
}
impl Application {
    pub(super) fn enable_local_loom(&mut self) {
        self.authorization.borrow_mut().targets.automatic = true;
        if let Some(profile) = profile() {
            if let Ok(Some(remote)) = crate::loom_remote_connection::Document::load(&profile) {
                self.local_loom.select(&remote.origin);
                self.authorization.borrow_mut().origin = Some(remote.origin);
            }
            match preference::load(&profile) {
                Ok(Some(saved)) if saved.paused => {
                    self.local_loom.select(&saved.origin);
                    let mut view = self.authorization.borrow_mut();
                    view.origin = Some(saved.origin);
                    view.status = "Loom 自动连接已暂停".into();
                }
                Ok(Some(saved)) => {
                    self.local_loom.select(&saved.origin);
                    self.local_loom.enable(profile);
                }
                Ok(None) => self.local_loom.enable(profile),
                Err(error) => self.authorization.borrow_mut().status = error,
            }
        }
    }
    pub(super) fn process_local_loom(&mut self) {
        let mut view = self.authorization.borrow_mut();
        if let Some(result) = self.local_loom.poll_disconnect() {
            view.status = match result {
                Ok(()) => "Loom 已断开".into(),
                Err(error) => format!("本地已断开，Loom 离线确认失败：{error:?}"),
            };
        }
        if matches!(view.request, Some(Request::Cancel)) {
            if let Some(origin) = view.origin.as_deref() {
                let now = crate::desktop::surface_windows::now_ms().unwrap_or(u64::MAX);
                if let Some(token) = self.authorization_owner.token(origin, now) {
                    match crate::device_authorization_config::Config::from_env(origin)
                        .map_err(|_| crate::loom_local_connection::Error::Store)
                        .and_then(|config| {
                            self.local_loom.disconnect(
                                config,
                                zeroize::Zeroizing::new(token.to_owned()),
                                self.execution_wake.callback(),
                            )
                        }) {
                        Ok(()) => {}
                        Err(error) => view.status = format!("Loom 断开请求失败：{error:?}"),
                    }
                }
            }
        }
        if let Some(Request::Select(origin)) = &view.request {
            let origin = origin.clone();
            // Reselecting the same managed service preserves existing renewal; restart only on reconnection after disconnect or submission of a new connection code.
            if !self.local_loom.enabled()
                || !self.local_loom.manages(&origin)
                || !view.connection_password.is_empty()
            {
                self.local_loom.stop();
                self.local_loom.select(&origin);
                self.local_loom
                    .password(std::mem::take(&mut view.connection_password));
                if let Some(profile) = profile() {
                    match preference::save(&profile, false, &origin) {
                        Ok(()) => self.local_loom.enable(profile),
                        Err(error) => view.status = error,
                    }
                }
            } else {
                // This selection is already satisfied; do not pass it to generic Select and clear the current connection label again.
                view.request = None;
            }
        }
        if view.closed || matches!(view.request, Some(Request::Cancel)) {
            self.local_loom.stop();
            if view.targets.automatic && matches!(view.request, Some(Request::Cancel)) {
                if let Some((profile, origin)) = profile().zip(view.origin.as_deref()) {
                    if let Err(error) = preference::save(&profile, true, origin) {
                        view.status = format!("本次连接已停止，但未保存停止状态：{error}");
                        // Main authorization handling must still consume Cancel to stop existing leases.
                    }
                }
            }
        }
        // Trusted local discovery already completed; menu renewal follows the same connection owner and must not permanently stop automatic renewal.
        let reconnect = matches!(view.request, Some(Request::Start(_)))
            && view
                .origin
                .as_deref()
                .is_some_and(|origin| self.local_loom.manages(origin));
        if reconnect {
            view.request = None;
            view.status = "正在连接 Loom…".into();
        } else if matches!(view.request, Some(Request::Start(_))) {
            self.local_loom.stop();
        }
        if matches!(view.request, Some(Request::ProjectionTargets))
            && view.origin.as_deref().is_some_and(|origin| {
                self.local_loom.enabled()
                    && self.local_loom.manages(origin)
                    && crate::desktop::surface_windows::now_ms()
                        .is_ok_and(|now| self.authorization_owner.token(origin, now).is_none())
            })
        {
            self.local_loom.refresh();
            view.request = None;
            view.targets.status = "正在连接 Loom，连接后自动发现设备…".into();
        }
        drop(view);
        if reconnect {
            let origin = self.authorization.borrow().origin.clone();
            if let Some((profile, origin)) = profile().zip(origin) {
                if let Err(error) = preference::save(&profile, false, &origin) {
                    self.authorization.borrow_mut().status = format!("连接设置未保存：{error}");
                    return;
                }
            }
            self.enable_local_loom();
        }
        let Some(result) = self.local_loom.poll(|| self.execution_wake.callback()) else {
            return;
        };
        let mut view = self.authorization.borrow_mut();
        match result {
            Ok(connected) => {
                if let Some(previous) = view
                    .origin
                    .as_deref()
                    .filter(|old| *old != connected.origin)
                {
                    let _ = self.authorization_owner.invalidate(previous);
                    self.remote_sessions.cancel_discovery();
                    view.catalog = None;
                }
                let result = crate::loom_local_connection::remember(&connected).and_then(|_| {
                    let now = crate::desktop::surface_windows::now_ms()
                        .map_err(|_| crate::loom_local_connection::Error::Stopped)?;
                    self.authorization_owner
                        .install(&connected.origin, connected.session, now)
                        .map_err(|_| crate::loom_local_connection::Error::DeviceDenied)
                });
                match result {
                    Ok(()) => {
                        self.local_loom.installed();
                        view.origin = Some(connected.origin);
                        view.busy = false;
                        view.status = "Loom 已连接".into();
                        view.connected = true;
                        view.loom_name = connected.name;
                        view.loom_id = Some(connected.loom_id);
                    }
                    Err(error) => {
                        view.status = format!("Loom 授权未安装：{error:?}");
                        self.local_loom.retry_install();
                    }
                }
            }
            Err(error) => {
                if error == crate::loom_local_connection::Error::AwaitingApproval {
                    view.status = "等待 Loom 管理员批准此设备…".into();
                    return;
                }
                if error == crate::loom_local_connection::Error::DeviceDenied {
                    if let Some(origin) = &view.origin {
                        let _ = self.authorization_owner.invalidate(origin);
                    }
                    view.status = "请在 Loom 批准此设备，然后重新连接；已禁用的设备须先启用".into();
                    return;
                }
                // Do not disguise failure as online status or overwrite the user's explicitly selected remote service.
                view.status = format!("Loom 未连接：{error:?}");
            }
        }
    }
}

//! 单个本机连接任务；UI 只收已完成结果，显式选择/取消后迟到结果不得安装。
use super::{Connected, Error};
use crate::device_request_cancel::Cancel;
use std::{
    path::PathBuf,
    sync::{mpsc, Arc},
    thread::JoinHandle,
    time::{Duration, Instant},
};
#[path = "loom_disconnect.rs"]
mod disconnect;

struct Pending {
    cancel: Arc<Cancel>,
    thread: JoinHandle<()>,
    result: mpsc::Receiver<Result<Connected, Error>>,
}
#[derive(Default)]
pub(crate) struct Owner {
    profile: Option<PathBuf>,
    pending: Option<Pending>,
    next: Option<Instant>,
    retry: u64,
    install_retry: u64,
    origin: Option<String>,
    disconnect: Option<disconnect::Job>,
    password: zeroize::Zeroizing<String>,
}
impl Owner {
    pub(crate) fn password(&mut self, password: zeroize::Zeroizing<String>) {
        self.password = password;
    }
    pub(crate) fn disconnect(
        &mut self,
        config: crate::device_authorization_config::Config,
        token: zeroize::Zeroizing<String>,
        wake: Arc<dyn Fn() + Send + Sync>,
    ) -> Result<(), Error> {
        if self.disconnect.is_some() {
            return Err(Error::Stopped);
        }
        self.disconnect = Some(disconnect::Job::start(config, token, wake)?);
        Ok(())
    }
    pub(crate) fn poll_disconnect(&mut self) -> Option<Result<(), Error>> {
        let result = self.disconnect.as_mut()?.poll()?;
        self.disconnect.take();
        Some(result)
    }
    pub(crate) fn select(&mut self, origin: &str) {
        self.origin = Some(origin.to_owned());
    }
    pub(crate) fn enable(&mut self, profile: PathBuf) {
        self.profile = Some(profile);
        self.next = self.pending.is_none().then(Instant::now);
        self.retry = 2;
        self.install_retry = 2;
    }
    pub(crate) fn stop(&mut self) {
        self.profile = None;
        self.password.clear();
        self.next = None;
        if let Some(pending) = &self.pending {
            pending.cancel.cancel();
        }
    }
    pub(crate) fn manages(&self, origin: &str) -> bool {
        self.origin.as_deref() == Some(origin)
    }
    pub(crate) fn refresh(&mut self) {
        if self.profile.is_some() && self.pending.is_none() {
            self.next = Some(Instant::now());
        }
    }
    pub(crate) fn enabled(&self) -> bool {
        self.profile.is_some()
    }
    pub(crate) fn installed(&mut self) {
        self.install_retry = 2;
    }
    pub(crate) fn retry_install(&mut self) {
        if self.profile.is_some() {
            self.next = Some(Instant::now() + Duration::from_secs(self.install_retry));
            self.install_retry = (self.install_retry * 2).min(30);
        }
    }
    pub(crate) fn deadline(&self) -> Option<Instant> {
        if self.disconnect.is_some() {
            None
        } else {
            self.next
        }
    }
    pub(crate) fn poll(
        &mut self,
        wake: Arc<dyn Fn() + Send + Sync>,
    ) -> Option<Result<Connected, Error>> {
        let mut output = None;
        let completed = self
            .pending
            .as_ref()
            .and_then(|p| match p.result.try_recv() {
                Ok(result) => Some(result),
                Err(mpsc::TryRecvError::Disconnected) => Some(Err(Error::Stopped)),
                Err(mpsc::TryRecvError::Empty) => None,
            });
        if let Some(result) = completed {
            let pending = self.pending.take().unwrap();
            let _ = pending.thread.join();
            if self.profile.is_some() && !pending.cancel.is_stopped() {
                let delay = match &result {
                    Ok(connected) => {
                        self.origin = Some(connected.origin.clone());
                        self.retry = 2;
                        let now = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .ok()
                            .and_then(|v| u64::try_from(v.as_millis()).ok())
                            .unwrap_or(u64::MAX);
                        Duration::from_millis(
                            connected
                                .session
                                .expires_at_ms()
                                .saturating_sub(now)
                                .saturating_sub(60_000)
                                .max(1_000),
                        )
                    }
                    Err(Error::DeviceDenied) => {
                        self.stop();
                        Duration::ZERO
                    }
                    Err(_) => {
                        let delay = Duration::from_secs(self.retry);
                        self.retry = (self.retry * 2).min(30);
                        delay
                    }
                };
                self.next = self.profile.as_ref().map(|_| Instant::now() + delay);
                output = Some(result);
            } else if self.profile.is_some() {
                // 清除后再次显式连接：先回收已取消任务，再启动新的身份校验。
                self.next = Some(Instant::now());
            }
        }
        // 断开撤销旧设备会话结束后才允许签发新会话，避免迟到的断开踢掉新连接。
        if self.disconnect.is_none()
            && self.pending.is_none()
            && self.next.is_some_and(|at| at <= Instant::now())
        {
            let profile = self.profile.as_ref()?.clone();
            let origin = self.origin.clone();
            let password = std::mem::take(&mut self.password);
            self.next = None;
            let cancel = Arc::new(Cancel::new());
            let stop = cancel.clone();
            let (send, result) = mpsc::sync_channel(1);
            match std::thread::Builder::new()
                .name("otter-local-loom".into())
                .spawn(move || {
                    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        if let Some(origin) = origin {
                            crate::loom_selected_connection::connect(
                                &origin, &profile, &stop, &password,
                            )
                        } else {
                            super::connect(
                                crate::loom_discovery::Config::from_env(),
                                &profile,
                                &stop,
                            )
                        }
                    }))
                    .unwrap_or(Err(Error::Worker));
                    // 先发布结果，再通知 UI；不能依赖通知时 JoinHandle 已标记 finished。
                    let _ = send.send(result);
                    wake();
                }) {
                Ok(thread) => {
                    self.pending = Some(Pending {
                        cancel,
                        thread,
                        result,
                    })
                }
                Err(_) => {
                    self.next = Some(Instant::now() + Duration::from_secs(30));
                    output = Some(Err(Error::Stopped));
                }
            }
        }
        output
    }
    pub(crate) fn shutdown(&mut self) {
        self.stop();
        self.disconnect.take();
        if let Some(pending) = self.pending.take() {
            let _ = pending.thread.join();
        }
    }
}
impl Drop for Owner {
    fn drop(&mut self) {
        self.shutdown();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn local_loom_reenable_does_not_spin_or_lose_cancelled_pending_work() {
        let (send, result) = mpsc::sync_channel(1);
        let cancel = Arc::new(Cancel::new());
        let worker_cancel = cancel.clone();
        let thread = std::thread::spawn(move || {
            while !worker_cancel.is_stopped() {
                std::thread::yield_now();
            }
            let _ = send.send(Err(Error::Stopped));
        });
        let mut owner = Owner::default();
        owner.pending = Some(Pending {
            cancel,
            thread,
            result,
        });
        owner.enable(PathBuf::from("test-profile"));
        assert!(owner.enabled());
        assert!(owner.deadline().is_none(), "pending worker owns the wakeup");
        owner.stop();
        owner.refresh();
        assert!(!owner.enabled());
        assert!(owner.deadline().is_none());
        owner.shutdown();
    }
    #[test]
    fn local_loom_install_retry_backs_off_independently_of_network_success() {
        let mut owner = Owner::default();
        owner.enable(PathBuf::from("test-profile"));
        for delay in [2, 4, 8, 16, 30, 30] {
            let before = Instant::now();
            owner.retry_install();
            assert!(owner.deadline().unwrap() >= before + Duration::from_secs(delay));
        }
        owner.installed();
        assert_eq!(owner.install_retry, 2);
    }
}

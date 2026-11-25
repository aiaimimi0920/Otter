//! One local connection task; UI receives only completed results, and late results after explicit selection/cancellation cannot install.
use super::{Connected, Error};
use crate::device_request_cancel::Cancel;
use std::{
    path::PathBuf,
    sync::Arc,
    time::{Duration, Instant},
};
#[path = "loom_disconnect.rs"]
mod disconnect;
#[path = "loom_connection_task.rs"]
mod task;
use task::Task;
#[derive(Default)]
pub(crate) struct Owner {
    profile: Option<PathBuf>,
    pending: Option<Task<Connected>>,
    next: Option<Instant>,
    retry: u64,
    install_retry: u64,
    origin: Option<String>,
    disconnect: Option<Task<()>>,
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
        self.disconnect = Some(disconnect::start(config, token, wake)?);
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
        self.pending
            .as_ref()
            .and_then(Task::deadline)
            .into_iter()
            .chain(self.disconnect.as_ref().and_then(Task::deadline))
            .chain(self.next.filter(|_| self.disconnect.is_none()))
            .min()
    }
    pub(crate) fn poll(
        &mut self,
        make_wake: impl FnOnce() -> Arc<dyn Fn() + Send + Sync>,
    ) -> Option<Result<Connected, Error>> {
        let mut output = None;
        if let Some(result) = self.pending.as_mut().and_then(Task::poll) {
            let pending = self.pending.take().unwrap();
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
                // Explicit connection after clearing: reclaim cancelled tasks before starting new identity validation.
                self.next = Some(Instant::now());
            }
        }
        // Issue a new session only after disconnect finishes revoking the old device session, preventing late disconnect from kicking the new connection.
        if self.disconnect.is_none()
            && self.pending.is_none()
            && self.next.is_some_and(|at| at <= Instant::now())
        {
            let profile = self.profile.as_ref()?.clone();
            let origin = self.origin.clone();
            let password = std::mem::take(&mut self.password);
            self.next = None;
            // Idle, deferred and in-flight polls need no per-worker callback.
            match Task::spawn("otter-local-loom", make_wake(), move |stop| {
                if let Some(origin) = origin {
                    crate::loom_selected_connection::connect(&origin, &profile, stop, &password)
                } else {
                    super::connect(crate::loom_discovery::Config::from_env(), &profile, stop)
                }
            }) {
                Ok(task) => self.pending = Some(task),
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
        self.pending.take();
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
    use crate::desktop::loom_execution_tasks::reap_fixture::{Gate, WAIT};

    #[test]
    fn local_loom_completion_notification_does_not_join_or_publish_cancelled_work() {
        let gate = Gate::new(|| {});
        let mut owner = Owner::default();
        owner.pending =
            Some(Task::spawn("connect-test", gate.notify, |_| Err(Error::Response)).unwrap());
        owner.enable("test-profile".into());
        let unexpected = || panic!("replacement started too soon");
        gate.noticed.recv_timeout(WAIT).unwrap();
        assert!(owner.poll(unexpected).is_none());
        assert!(owner.deadline().is_some());
        owner.stop();
        assert!(owner.poll(unexpected).is_none());
        gate.release.send(()).unwrap();
        owner.pending.as_ref().unwrap().test_wait_finished();
        assert!(owner.poll(unexpected).is_none(), "cancelled result escaped");
        assert!(owner.pending.is_none());
        assert!(owner.deadline().is_none());
    }

    #[test]
    fn local_loom_disconnect_keeps_reconnect_gated_and_notifies_even_on_panic() {
        for panic in [false, true] {
            let gate = Gate::new(|| {});
            let mut owner = Owner::default();
            owner.disconnect = Some(
                Task::spawn("disconnect-test", gate.notify, move |_| {
                    assert!(!panic, "injected disconnect failure");
                    Ok(())
                })
                .unwrap(),
            );
            owner.enable("test-profile".into());
            gate.noticed.recv_timeout(WAIT).unwrap();
            assert!(owner.poll_disconnect().is_none());
            assert!(owner.deadline().is_some());
            assert!(owner.poll(|| panic!("overlapping connection")).is_none());
            gate.release.send(()).unwrap();
            owner.disconnect.as_ref().unwrap().test_wait_finished();
            assert_eq!(
                owner.poll_disconnect(),
                Some(if panic { Err(Error::Worker) } else { Ok(()) })
            );
            assert!(owner.poll_disconnect().is_none());
            assert!(owner.deadline().is_some_and(|due| due <= Instant::now()));
            owner.stop();
            assert!(owner.deadline().is_none());
        }
    }

    #[test]
    fn local_loom_reenable_does_not_spin_or_lose_cancelled_pending_work() {
        let mut owner = Owner::default();
        owner.pending = Some(
            Task::spawn("local-loom-test", Arc::new(|| {}), |cancel| {
                while !cancel.is_stopped() {
                    std::thread::yield_now();
                }
                Err(Error::Stopped)
            })
            .unwrap(),
        );
        owner.enable(PathBuf::from("test-profile"));
        assert!(owner.enabled());
        assert!(owner.deadline().is_none(), "pending worker owns the wakeup");
        owner.stop();
        owner.refresh();
        assert!(!owner.enabled());
        owner.shutdown();
        assert!(owner.deadline().is_none());
    }
    #[test]
    fn local_loom_idle_poll_does_not_create_worker_wake() {
        let mut owner = Owner::default();
        let unexpected = || panic!("idle poll constructed a worker callback");
        assert!(owner.poll(unexpected).is_none());
        owner.enable(PathBuf::from("test-profile"));
        owner.retry_install();
        assert!(owner.poll(unexpected).is_none());
        assert!(owner.deadline().is_some());
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

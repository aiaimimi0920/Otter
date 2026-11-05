//! Session 的输入 owner：显式启用、异步结算，输入释放完成前不退休源保护。
use super::Session;
use crate::{
    live_input::{Event, MAX_SEQUENCE},
    live_input_worker::{Command, Reason, Worker},
    live_window_target::input_backend::Backend,
};
use std::{io, os::windows::io::BorrowedHandle};
impl Session {
    pub(super) fn process_window_input(&mut self) {
        if self.remote_input.is_some() {
            // 网络输入仍由同一worker处理，但不把远程资格暴露成本地窗口的鼠标/键盘控制。
            if let Some(window) = &self.window {
                window.take_input_request();
                while window.pop_input().is_some() {}
                window.input_status(false, false, None);
            }
            return;
        }
        let request = self
            .window
            .as_ref()
            .and_then(|window| window.take_input_request());
        if let Some(enabled) = request {
            if enabled {
                if let Err(error) = self.enable_input() {
                    self.input_error.get_or_insert_with(|| error.to_string());
                }
            } else {
                self.disable_input();
            }
        }
        while let Some(event) = self.window.as_ref().and_then(|window| window.pop_input()) {
            let sequence = self.last_input_sequence.saturating_add(1);
            if let Err(error) = self.send_input(sequence, event) {
                self.input_error.get_or_insert_with(|| error.to_string());
                self.disable_input();
                break;
            }
        }
        if let Some(window) = &self.window {
            let pending =
                !self.input_enabled && self.input.as_ref().is_some_and(|worker| !worker.stopping());
            let stop = self.input.as_ref().and_then(Worker::stop_signal);
            window.input_status(self.input_enabled, pending, stop);
            if self.input_error.is_some() && !self.input_enabled && !pending {
                window.input_failure();
            }
        }
    }
    pub fn enable_input(&mut self) -> io::Result<()> {
        if self.remote_input.is_some() {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "Remote Live owns input",
            ));
        }
        if self
            .window
            .as_ref()
            .is_some_and(|window| window.group_input_blocked())
        {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "Live group is locked or hidden",
            ));
        }
        if self.stopping
            || self.window.as_ref().is_none_or(|window| !window.is_alive())
            || self.capture_result.is_some()
        {
            return Err(io::Error::other("Live session is closing"));
        }
        if self.input.is_some() {
            return Err(io::Error::new(
                io::ErrorKind::WouldBlock,
                "Live input owner already active or draining",
            ));
        }
        let target = self.target.clone().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::PermissionDenied,
                "Region Live cannot control a window",
            )
        })?;
        let region = self.input_crop.map(|crop| crop.input());
        let mut worker = Worker::start_for_ui(move || {
            Backend::new(target, region).map_err(|error| format!("{error:?}"))
        })?;
        let submitted = worker.submit(Command::Enable);
        self.input = Some(worker);
        self.input_enabled = false;
        self.input_error = None;
        self.input_cleanup = None;
        self.input_cleanup_unknown = false;
        if let Err(error) = submitted {
            self.disable_input();
            return Err(io::Error::other(format!("{error:?}")));
        }
        Ok(())
    }
    pub fn input_enabled(&self) -> bool {
        self.input_enabled
    }
    pub fn take_input_error(&mut self) -> Option<String> {
        self.input_error.take()
    }
    pub fn input_cleanup(&self) -> Option<crate::live_input::Cleanup> {
        self.input_cleanup
    }
    pub fn input_completion_handle(&self) -> Option<BorrowedHandle<'_>> {
        self.input.as_ref().and_then(Worker::completion_handle)
    }
    pub fn send_input(&mut self, sequence: u64, event: Event) -> io::Result<()> {
        if !self.input_enabled
            || self
                .window
                .as_ref()
                .is_some_and(|window| window.group_input_blocked())
        {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "Live interaction not enabled",
            ));
        }
        if sequence == 0 || sequence > MAX_SEQUENCE || sequence <= self.last_input_sequence {
            self.disable_input();
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Live input sequence rejected",
            ));
        }
        let result = self
            .input
            .as_mut()
            .ok_or_else(|| io::Error::other("Live input owner missing"))?
            .submit(Command::Input { sequence, event });
        match result {
            Ok(()) => {
                self.last_input_sequence = sequence;
                Ok(())
            }
            Err(error) => {
                self.disable_input();
                Err(io::Error::other(format!("{error:?}")))
            }
        }
    }
    pub fn disable_input(&mut self) {
        self.input_enabled = false;
        if let Some(worker) = &mut self.input {
            worker.stop();
        }
    }
    pub(super) fn poll_input(&mut self) {
        let Some(worker) = &mut self.input else {
            return;
        };
        loop {
            match worker.poll_reply() {
                Ok(Some(reply)) => match reply.result {
                    Ok(()) => {
                        if matches!(reply.command, Command::Enable)
                            && !worker.stopping()
                            && !self.stopping
                        {
                            self.input_enabled = true;
                        }
                    }
                    Err(error) => {
                        self.input_error.get_or_insert_with(|| format!("{error:?}"));
                        self.input_enabled = false;
                        worker.stop();
                    }
                },
                Ok(None) => break,
                Err(error) => {
                    self.input_error.get_or_insert_with(|| error.to_string());
                    self.input_enabled = false;
                    worker.stop();
                    break;
                }
            }
        }
        if let Some(exit) = worker.take_exit() {
            self.input_cleanup = exit.cleanup;
            self.input_cleanup_unknown = exit.cleanup.is_none();
            if exit.cleanup.is_none_or(|report| report.failed > 0)
                && (self.stopping || self.window.as_ref().is_none_or(|window| !window.is_alive()))
            {
                self.error
                    .get_or_insert_with(|| "Live input cleanup failed or outcome unknown".into());
            }
            self.input_enabled = false;
            if exit.reason != Reason::Stopped {
                self.input_error
                    .get_or_insert_with(|| format!("Live input stopped: {:?}", exit.reason));
            }
            match exit.cleanup {
                Some(report) if report.failed > 0 => {
                    self.input_error.get_or_insert_with(|| {
                        format!("Live input release failed: {}", report.failed)
                    });
                }
                None => {
                    self.input_error
                        .get_or_insert_with(|| "Live input cleanup outcome unknown".into());
                }
                _ => {}
            }
            self.input = None;
        }
    }
}

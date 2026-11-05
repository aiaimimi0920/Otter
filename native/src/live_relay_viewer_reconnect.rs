//! 同一授权媒体链的有限恢复；不续签、不加入、不更换epoch、不重放控制命令。
use super::*;
use crate::live_relay_viewer_frames::Cursor;
use std::time::{Duration, Instant};

pub(crate) struct Recovery {
    prepare: Box<dyn Fn(Cursor) -> Result<Prepared, ()> + Send>,
    notify: Box<dyn Fn(bool) + Send>,
    attempts: u32,
    watermark: Cursor,
    recovering: bool,
}
impl Recovery {
    pub(crate) fn new(
        epoch: u64,
        prepare: impl Fn(Cursor) -> Result<Prepared, ()> + Send + 'static,
        notify: impl Fn(bool) + Send + 'static,
    ) -> Self {
        Self {
            prepare: Box::new(prepare),
            notify: Box::new(notify),
            attempts: 0,
            watermark: Cursor { epoch, frame_id: 0 },
            recovering: false,
        }
    }
    pub(super) fn progress(&mut self, cursor: Cursor) {
        if cursor.epoch == self.watermark.epoch && cursor.frame_id > self.watermark.frame_id {
            self.watermark = cursor;
            self.attempts = 0;
            if self.recovering {
                self.recovering = false;
                (self.notify)(false);
            }
        }
    }
    pub(super) fn retry(
        &mut self,
        exit: Exit,
        control: &Control,
        cursor: Cursor,
    ) -> Result<Prepared, Exit> {
        if !matches!(exit, Exit::Transport(error) if error.recoverable()) {
            return Err(exit);
        }
        self.progress(
            control
                .session
                .committed(&control.permit)
                .map_err(|_| Exit::Stopped)?,
        );
        // 一个无像素进度的故障周期最多四次，握手成功/重复预热帧不刷新次数。
        if self.attempts >= 4 {
            return Err(exit);
        }
        self.recovering = true;
        (self.notify)(true);
        let delay = Duration::from_millis((400u64 << self.attempts).min(2000));
        self.attempts += 1;
        control.wait(delay)?;
        let prepared = (self.prepare)(cursor).map_err(|_| Exit::AuthorizationEnded)?;
        if control.stop.load(Ordering::Acquire)
            || control.session.validate(&control.permit).is_err()
        {
            return Err(Exit::Stopped);
        }
        Ok(prepared)
    }
}
impl Control {
    pub(super) fn wait(&self, delay: Duration) -> Result<(), Exit> {
        let deadline = Instant::now() + delay;
        loop {
            if self.stop.load(Ordering::Acquire) || self.session.validate(&self.permit).is_err() {
                return Err(Exit::Stopped);
            }
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return Ok(());
            }
            std::thread::park_timeout(remaining);
        }
    }
}

#[cfg(test)]
#[path = "live_relay_viewer_reconnect_tests.rs"]
mod tests;

//! 连续采集的有界调度状态；只在上一帧成功接受后允许下一步，不自动重试失败输入。
use crate::long_capture::{Append, Completion};
use std::time::{Duration, Instant};
#[derive(Default)]
pub(super) struct Auto {
    started: Option<Instant>,
    steps: u32,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Next {
    Stop,
    Preview,
    Scroll,
}
impl Auto {
    pub fn running(&self) -> bool {
        self.started.is_some()
    }
    pub fn start(&mut self, now: Instant) {
        self.started = Some(now);
        self.steps = 0;
    }
    pub fn stop(&mut self) {
        self.started = None;
    }
    pub fn request(&mut self, now: Instant) -> bool {
        let Some(started) = self.started else {
            return false;
        };
        if self.steps >= 511 || now.saturating_duration_since(started) >= Duration::from_secs(300) {
            self.stop();
            return false;
        }
        self.steps += 1;
        true
    }
    pub fn completed(&mut self, completion: &Completion, can_preview: bool) -> Next {
        if !self.running() {
            return Next::Stop;
        }
        match completion {
            Completion::Append(Ok(Append::Added { .. })) => {
                if can_preview {
                    Next::Preview
                } else {
                    Next::Scroll
                }
            }
            Completion::SpatialAppend(Ok(crate::long_capture::SpatialAppend::Added { .. })) => {
                Next::Preview
            }
            Completion::Preview(Ok(_)) | Completion::SpatialPreview(Ok(_)) => Next::Scroll,
            _ => {
                self.stop();
                Next::Stop
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn steps_time_duplicate_failure_and_user_stop_bound_the_loop() {
        let now = Instant::now();
        let mut auto = Auto::default();
        assert!(!auto.request(now));
        auto.start(now);
        for _ in 0..511 {
            assert!(auto.request(now));
            assert_eq!(
                auto.completed(&Completion::Append(Ok(Append::Added { origin: 1 })), false),
                Next::Scroll
            );
        }
        assert!(!auto.request(now));
        auto.start(now);
        assert!(!auto.request(now + Duration::from_secs(300)));
        for result in [
            Completion::Append(Ok(Append::Duplicate)),
            Completion::CaptureFailed("fixture".into()),
            Completion::WorkerStopped,
        ] {
            auto.start(now);
            assert_eq!(auto.completed(&result, true), Next::Stop);
            assert!(!auto.running());
        }
        auto.start(now);
        auto.stop();
        assert_eq!(
            auto.completed(&Completion::Append(Ok(Append::Added { origin: 1 })), true),
            Next::Stop
        );
    }
    #[test]
    fn confirmed_frames_preview_before_next_scroll_and_stop_invalidates_continuation() {
        let mut auto = Auto::default();
        auto.start(Instant::now());
        let added = Completion::Append(Ok(Append::Added { origin: 1 }));
        assert_eq!(auto.completed(&added, false), Next::Scroll);
        assert_eq!(auto.completed(&added, true), Next::Preview);
        let preview = Completion::Preview(Ok(crate::long_capture::Composite {
            width: 1,
            height: 1,
            origin: 0,
            pixels: vec![0; 4].into(),
        }));
        assert_eq!(auto.completed(&preview, true), Next::Scroll);
        auto.stop();
        assert_eq!(auto.completed(&preview, true), Next::Stop);
        auto.start(Instant::now());
        assert_eq!(
            auto.completed(
                &Completion::Preview(Err(crate::long_capture::Error::Budget)),
                true
            ),
            Next::Stop
        );
        assert!(!auto.running());
    }
}

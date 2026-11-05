//! 后台执行结束后的有界补偿；不创建重试队列/线程，不把失败租约伪装成成功。
use super::{Completion, Failure};
use crate::loom_art_control::{self, Outcome};
use std::{thread, time::Duration};
impl Completion {
    fn cancellation_pending(&self) -> bool {
        matches!(
            &self.remote_cancel,
            Some(Err(_)) | Some(Ok(Outcome::Failed | Outcome::Cancelled))
        )
    }
    pub fn cleanup_pending(&self) -> bool {
        self.cancellation_pending()
            || matches!(&self.result,Err(Failure::PendingRelease(delivery)) if delivery.pending_release_count()>0)
    }
    /// 首次失败后最多两次补偿，等待发生在原执行线程，未完成结果仍由 Completion 持有。
    pub(super) fn retry_cleanup(&mut self) {
        for delay in [Duration::from_millis(100), Duration::from_millis(500)] {
            if !self.cleanup_pending() {
                break;
            }
            thread::sleep(delay);
            self.cleanup_retries += 1;
            if self.cancellation_pending() {
                self.remote_cancel = Some(loom_art_control::cancel_authenticated(
                    &self.identity,
                    &self.unit,
                    &self.request,
                ));
            }
            if let Err(Failure::PendingRelease(delivery)) = &mut self.result {
                if delivery.pending_release_count() > 0 {
                    let _ = delivery.release_authenticated(&self.identity);
                }
            }
        }
    }
}

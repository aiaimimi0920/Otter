//! 应用资源结算：精确路由 Ready，轮转窗口请求，不让单个场景占满共享容量。
use super::*;
use crate::surface_resource_runtime::{request::Request, Ready};
pub(crate) enum StartImageError {
    Busy,
    Failed(String),
}
impl Windows {
    pub(crate) fn install_ready(&mut self, ready: Ready, now: u64) {
        let Some(window) = self.entries.iter_mut().find(|w| {
            w.is_alive()
                && w.binding() == ready.binding()
                && w.snapshot_revision() == ready.revision()
        }) else {
            return;
        }; // 过时结果消费释放，不永久留在 Ready 队列占容量。
        if let Err(error) = window.install_resource(ready, now) {
            window.record_image_error(&format!("图片安装失败：{error:?}"));
        }
    }
    pub(crate) fn schedule_images(
        &mut self,
        now: u64,
        mut start: impl FnMut(Request) -> std::result::Result<(), StartImageError>,
    ) {
        if self.entries.is_empty() {
            return;
        }
        let first = self.image_cursor % self.entries.len();
        self.image_cursor = (first + 1) % self.entries.len();
        let (before, after) = self.entries.split_at_mut(first);
        for window in after.iter_mut().chain(before.iter_mut()) {
            // 无效候选没有完成句柄：本轮有界跳过，不能让后续有效候选永远等不到唤醒。
            for _ in 0..64 {
                let request = match window.next_image_request(now) {
                    Ok(Some(request)) => request,
                    Ok(None) => break,
                    Err(error) => {
                        window.record_image_error(&format!("图片请求无效：{error:?}"));
                        continue;
                    }
                };
                match start(request) {
                    Ok(()) => {
                        window.finish_image_request();
                        break;
                    }
                    Err(StartImageError::Busy) => break,
                    Err(StartImageError::Failed(error)) => {
                        window.finish_image_request();
                        window.record_image_error(&error);
                    }
                }
            }
        }
    }
}

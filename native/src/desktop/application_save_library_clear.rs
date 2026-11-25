//! 清空沿用原库更新 worker、取消和 Receipt 发布；不创建磁盘删除任务。
use super::*;
impl Job {
    pub fn start_library_clear(
        source: crate::session_source::Source,
        kind: crate::session_library::Kind,
        notify: Arc<dyn Fn() + Send + Sync>,
    ) -> std::io::Result<Self> {
        Self::start_library(notify, move |cancel| {
            crate::session_library::remove::clear(&source, kind, cancel)
        })
    }
}

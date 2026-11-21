//! 清空沿用原库更新 worker、取消和 Receipt 发布；不创建磁盘删除任务。
use super::*;
impl Job {
    pub fn start_library_clear(
        source: crate::session_source::Source,
        kind: crate::session_library::Kind,
        notify: Arc<dyn Fn() + Send + Sync>,
    ) -> std::io::Result<Self> {
        let mut job = Self::start_with(notify, move |cancel| {
            let revision = crate::session_document::decode(&source)
                .map_err(|error| format!("Invalid frozen clear source: {error:?}"))?
                .revision;
            let bytes = crate::session_library::remove::clear(&source, kind, cancel)?;
            Ok(Output {
                checkpoint_accounts: None,
                checkpoint_senders: None,
                checkpoint_scope: None,
                checkpoint_projections: None,
                library_changed: bytes.as_ref() != source.as_ref(),
                candidate: Candidate {
                    bytes,
                    expected_revision: revision,
                    revision,
                },
                staged: None,
            })
        })?;
        job.purpose = Purpose::LibraryUpdate;
        Ok(job)
    }
}

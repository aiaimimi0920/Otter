//! 关闭前回收编码：共享单 worker 生命周期，不销毁窗口、不发布原文或暂存磁盘。
use super::{Data, Job, Output, Purpose};
use crate::{session_library::recycle, session_write::Candidate, workspace_id::UnitId};
use std::sync::Arc;

pub struct Request {
    pub unit: UnitId,
    pub entry_id: String,
    pub created_at: String,
    pub policy: recycle::policy::Policy,
    pub now_ms: i64,
}
impl Job {
    pub fn start_recycle(
        data: Data,
        request: Request,
        notify: Arc<dyn Fn() + Send + Sync>,
    ) -> std::io::Result<Self> {
        let unit = request.unit.clone();
        let mut job = Self::start_with(notify, move |cancel| {
            let original = data.original.clone();
            let revision = crate::session_document::decode(&original)
                .map_err(|error| format!("Invalid recycle source: {error:?}"))?
                .revision;
            // 必须编码当前模型；旧磁盘快照只用于保留非目标窗口和未知字段。
            let snapshot = data
                .encode(cancel)
                .map_err(|error| format!("Recycle snapshot: {error:?}"))?;
            let bytes = recycle::prepare(
                &original,
                recycle::Request {
                    snapshot: &snapshot.bytes,
                    unit: &request.unit,
                    entry_id: &request.entry_id,
                    created_at: &request.created_at,
                },
                request.policy,
                request.now_ms,
                cancel,
            )?;
            Ok(Output {
                checkpoint_senders: None,
                checkpoint_scope: None,
                checkpoint_projections: None,
                library_changed: true,
                candidate: Candidate {
                    bytes,
                    expected_revision: revision,
                    revision,
                },
                staged: None,
            })
        })?;
        job.purpose = Purpose::Recycle(unit);
        Ok(job)
    }
}

impl Job {
    pub fn start_prune(
        source: crate::session_source::Source,
        policy: recycle::policy::Policy,
        now_ms: i64,
        notify: Arc<dyn Fn() + Send + Sync>,
    ) -> std::io::Result<Self> {
        let mut job = Self::start_with(notify, move |cancel| {
            let revision = crate::session_document::decode(&source)
                .map_err(|error| format!("Invalid prune source: {error:?}"))?
                .revision;
            let candidate =
                crate::session_library::prune::prepare(&source, policy, now_ms, cancel)?;
            let library_changed = candidate.is_some();
            let bytes = match candidate {
                Some(bytes) => bytes,
                None => match &source {
                    crate::session_source::Source::Encoded(bytes) => bytes.clone(),
                    crate::session_source::Source::Imported(_) => {
                        // 未变更的导入原文只做有界传递，不在提交时更换其共享来源或置脏。
                        let raw: &serde_json::value::RawValue =
                            serde_json::from_slice(&source).map_err(|error| error.to_string())?;
                        crate::session_write::encode_bounded(&raw, cancel)
                            .map_err(|error| format!("Prune source encoding: {error:?}"))?
                    }
                },
            };
            Ok(Output {
                checkpoint_senders: None,
                checkpoint_scope: None,
                checkpoint_projections: None,
                candidate: Candidate {
                    bytes,
                    expected_revision: revision,
                    revision,
                },
                staged: None,
                library_changed,
            })
        })?;
        job.purpose = Purpose::LibraryUpdate;
        job.prune_policy = Some(policy);
        Ok(job)
    }
}

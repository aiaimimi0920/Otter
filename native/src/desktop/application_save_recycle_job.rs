//! 关闭前回收编码：共享单 worker 生命周期，不销毁窗口、不发布原文或暂存磁盘。
use super::{FrozenData, Job, Output, Purpose};
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
        data: FrozenData,
        request: Request,
        notify: Arc<dyn Fn() + Send + Sync>,
    ) -> std::io::Result<Self> {
        let unit = request.unit.clone();
        let mut job = Self::start_with(notify, move |cancel| {
            let original = data.original().clone();
            let source = crate::session_document::decode(&original)
                .map_err(|error| format!("Invalid recycle source: {error:?}"))?;
            let revision = source.revision;
            // 必须编码当前模型；旧磁盘快照只用于保留非目标窗口和未知字段。
            let snapshot = data
                .encode(&source, &request.unit, cancel)
                .map_err(|error| format!("Recycle snapshot: {error:?}"))?;
            let bytes = recycle::prepare(
                &original,
                recycle::Request {
                    snapshot: &snapshot,
                    unit: &request.unit,
                    entry_id: &request.entry_id,
                    created_at: &request.created_at,
                },
                request.policy,
                request.now_ms,
                cancel,
            )?;
            Ok(Output::unstaged(Candidate {
                bytes,
                expected_revision: revision,
                revision,
            }))
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
        let mut job = Self::start_library(notify, move |cancel| {
            crate::session_library::prune::prepare(&source, policy, now_ms, cancel)
        })?;
        job.prune_policy = Some(policy);
        Ok(job)
    }
}

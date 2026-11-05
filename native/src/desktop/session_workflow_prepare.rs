//! 恢复候选到真实工作流快照：参数策略和原生烘焙串接，编码租约不早于快照释放。
use super::*;
use crate::session_document::workflow_snapshot::{self, content, Snapshot};
#[path = "session_workflow_job.rs"]
pub mod job;

pub struct Prepared {
    // 字段释放顺序是合同：包含 Data URL 的快照先释放，预算租约随后释放。
    snapshot: Snapshot,
    _leases: Vec<crate::byte_budget::Lease>,
}
impl Prepared {
    pub fn snapshot(&self) -> &Snapshot {
        &self.snapshot
    }
}
impl VectorCandidate {
    /// 工作线程调用；候选来自已保存版本，但此方法不证明持久化/授权，不发送网络请求。
    pub fn prepare_workflow_snapshot(
        &self,
        catalog: &[serde_json::Value],
        cancel: &AtomicBool,
    ) -> Result<Prepared> {
        self.prepare_workflow_snapshot_with_root(catalog, None, cancel)
    }
    pub fn prepare_workflow_snapshot_with_root(
        &self,
        catalog: &[serde_json::Value],
        root: Option<&crate::desktop::session_asset_root::Root>,
        cancel: &AtomicBool,
    ) -> Result<Prepared> {
        check(cancel)?;
        if self.vectors.len() != self.candidate.placements.len()
            || self.vectors.iter().any(|(_, restored)| {
                restored.editor.revision() != 0 || restored.editor.draft().is_some()
            })
        {
            return Err(Error::new(
                E_ABORT,
                "Workflow candidate changed after restoration",
            ));
        }
        let mut leases = Vec::new();
        leases
            .try_reserve_exact(self.vectors.len())
            .map_err(|_| Error::new(E_INVALIDARG, "Cannot reserve workflow image ownership"))?;
        let snapshot = workflow_snapshot::sources::with_sources(
            self.candidate.original_json(),
            catalog,
            cancel,
            |document, sources| {
                content::prepare(
                    self.candidate.original_json(),
                    self.candidate.revision,
                    catalog,
                    cancel,
                    |window, _, _| {
                        let index = document
                            .windows
                            .iter()
                            .position(|item| item.id == window.id)
                            .ok_or(workflow_snapshot::Error::Node)?;
                        let reference = sources[index].ok_or(workflow_snapshot::Error::Content)?;
                        let images = self
                            .candidate
                            .images
                            .get(index)
                            .filter(|images| images.unit.as_str() == window.id)
                            .ok_or(workflow_snapshot::Error::Node)?;
                        let existing = if window.src.as_deref() == Some(reference) {
                            images.source.as_ref()
                        } else if window.preview_src.as_deref() == Some(reference) {
                            images.preview.as_ref()
                        } else {
                            None
                        };
                        let loaded;
                        let selected = match existing {
                            Some(image) => image,
                            None => {
                                loaded = crate::desktop::session_prepare::workflow_image(
                                    window, reference, root, cancel,
                                )
                                .map_err(|_| {
                                    if cancel.load(Ordering::Acquire) {
                                        workflow_snapshot::Error::Cancelled
                                    } else {
                                        workflow_snapshot::Error::Content
                                    }
                                })?;
                                &loaded
                            }
                        };
                        let png = self
                            .bake_window_image_png(window, Some(selected), cancel)
                            .map_err(|_| {
                                if cancel.load(Ordering::Acquire) {
                                    workflow_snapshot::Error::Cancelled
                                } else {
                                    workflow_snapshot::Error::Content
                                }
                            })?;
                        let encoded = crate::loom_base64::encode_png_data_url(&png, cancel)
                            .map_err(|error| {
                                if error == crate::loom_base64::Error::Cancelled {
                                    workflow_snapshot::Error::Cancelled
                                } else {
                                    workflow_snapshot::Error::Budget
                                }
                            })?;
                        let (text, lease) = encoded.into_parts();
                        leases.push(lease);
                        Ok(text)
                    },
                )
            },
        )
        .map_err(|error| {
            Error::new(
                if error == workflow_snapshot::Error::Cancelled {
                    E_ABORT
                } else {
                    E_INVALIDARG
                },
                format!("Workflow content preparation failed: {error:?}"),
            )
        })?;
        check(cancel)?;
        Ok(Prepared {
            snapshot,
            _leases: leases,
        })
    }
}

#[cfg(test)]
#[path = "session_workflow_source_pixels_tests.rs"]
mod source_pixels_tests;
#[cfg(test)]
#[path = "session_workflow_prepare_tests.rs"]
mod tests;

//! 单槽网络调度的有界目录；失败最多30秒退避，快速修改不增加任务队列。
use super::*;
#[derive(Default)]
pub(in crate::desktop::application::projection_sender) struct Schedule {
    pub(super) entries: Vec<Entry>,
    pub(in crate::desktop::application::projection_sender) wake: Option<Instant>,
}
#[cfg(test)]
#[path = "application_projection_sender_update_schedule_tests.rs"]
mod tests;
pub(super) struct Entry {
    pub(super) request: Request,
    pub(super) binding: Binding,
    pub(super) due: Instant,
    pub(super) stamp: Option<scope::ContentStamp>,
    failures: u8,
}
impl Schedule {
    pub(in crate::desktop::application::projection_sender) fn source_cached(
        &self,
        unit: &crate::workspace_id::UnitId,
        projection: &str,
        valid: impl FnOnce(&scope::ContentStamp) -> bool,
    ) -> bool {
        self.entries
            .iter()
            .find(|entry| {
                entry.binding.unit() == unit.as_str()
                    && entry.binding.envelope().projection_id() == projection
            })
            .and_then(|entry| entry.stamp.as_ref())
            .is_some_and(valid)
    }
    pub(in crate::desktop::application::projection_sender) fn source_settled(
        &mut self,
        unit: &crate::workspace_id::UnitId,
        projection: Option<&str>,
        stamp: Option<scope::ContentStamp>,
        success: bool,
        now: Instant,
    ) {
        if let Some(entry) = self.entries.iter_mut().find(|entry| {
            entry.binding.unit() == unit.as_str()
                && projection == Some(entry.binding.envelope().projection_id())
        }) {
            entry.stamp = stamp;
            entry.failures = if success {
                0
            } else {
                entry.failures.saturating_add(1)
            };
            entry.due = now
                + Duration::from_millis(if success {
                    750
                } else {
                    (500u64 << entry.failures.min(6)).min(30_000)
                });
        }
    }
    #[cfg(test)]
    pub(in crate::desktop::application::projection_sender) fn make_due(&mut self) {
        for entry in &mut self.entries {
            entry.due = Instant::now();
        }
    }
    pub(in crate::desktop::application::projection_sender) fn clear(&mut self) {
        self.entries.clear();
        self.wake = None;
    }
    pub(super) fn reconcile(&mut self, candidates: Vec<(Request, Binding)>, now: Instant) {
        self.entries.retain(|entry| {
            candidates.iter().any(|(request, binding)| {
                request.same_intent(&entry.request)
                    && binding == &entry.binding
                    && !binding.stopped()
            })
        });
        for (request, binding) in candidates.into_iter().take(64) {
            if binding.stopped()
                || self
                    .entries
                    .iter()
                    .any(|entry| entry.request.same_intent(&request))
            {
                continue;
            }
            self.entries.push(Entry {
                request,
                binding,
                due: now + Duration::from_millis(750),
                stamp: None,
                failures: 0,
            });
        }
    }
    pub(super) fn due(&self, now: Instant) -> Option<usize> {
        self.entries
            .iter()
            .enumerate()
            .filter(|(_, entry)| entry.due <= now)
            .min_by_key(|(_, entry)| entry.due)
            .map(|(index, _)| index)
    }
    pub(super) fn settle(
        &mut self,
        request: &Request,
        stamp: Option<scope::ContentStamp>,
        now: Instant,
    ) {
        if let Some(entry) = self
            .entries
            .iter_mut()
            .find(|entry| entry.request.same_intent(request))
        {
            let success = stamp.is_some();
            entry.stamp = stamp;
            entry.failures = if success {
                0
            } else {
                entry.failures.saturating_add(1)
            };
            entry.due = now
                + Duration::from_millis(if success {
                    750
                } else {
                    (500u64 << entry.failures.min(6)).min(30_000)
                });
        }
    }
}

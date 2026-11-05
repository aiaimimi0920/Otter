//! 暂存接管保留原 Editor（含重做分支），首次实际编辑结算为一个历史步骤。
use super::*;

#[derive(Debug, PartialEq, Eq)]
pub enum AdoptionOutcome {
    Pending,
    Cancelled,
    Committed,
    Superseded,
}

/// 宿主在草稿及异步编辑任务全部结束后调用 finish；未结算时不能丢弃此 owner。
pub struct Adoption {
    original: Option<Box<Editor>>,
    identity: std::sync::Arc<()>,
    revision: u64,
}

impl Editor {
    pub fn begin_adoption(&mut self, source: &Editor) -> Result<Option<Adoption>, EditError> {
        if self.draft.is_some() || source.draft.is_some() {
            return Ok(None);
        }
        let points = source
            .elements
            .iter()
            .map(|stroke| stroke.points.len())
            .sum::<usize>();
        if source.elements.len() > MAX_ELEMENTS
            || self
                .retained_points
                .checked_add(points)
                .is_none_or(|total| total > MAX_TOTAL_POINTS)
        {
            return Err(EditError::BudgetExceeded);
        }
        let mut candidate = source.export_snapshot();
        candidate.style = self.style;
        // 原文档仍占用点数，临时编辑的准入必须同时计入，不能绕过全局文档预算。
        candidate.retained_points += self.retained_points;
        let identity = candidate.identity.clone();
        let revision = candidate.revision;
        let original = Box::new(std::mem::replace(self, candidate));
        Ok(Some(Adoption {
            original: Some(original),
            identity,
            revision,
        }))
    }
}

impl Adoption {
    pub fn unchanged(&self, editor: &Editor) -> bool {
        self.original.is_some()
            && std::sync::Arc::ptr_eq(&self.identity, &editor.identity)
            && self.revision == editor.revision
    }
    pub fn finish(&mut self, editor: &mut Editor) -> Result<AdoptionOutcome, EditError> {
        if self.original.is_none() || !std::sync::Arc::ptr_eq(&self.identity, &editor.identity) {
            self.original.take();
            return Ok(AdoptionOutcome::Superseded);
        }
        if editor.draft.is_some() {
            return Ok(AdoptionOutcome::Pending);
        }
        let original = self.original.as_mut().unwrap();
        let outcome = if editor.revision == self.revision {
            AdoptionOutcome::Cancelled
        } else {
            // 在修改 original 之前完成预算检查；失败保留两份状态及 owner，允许宿主处理。
            if !original.adopt_snapshot(original.revision(), editor)? {
                return Ok(AdoptionOutcome::Pending);
            }
            AdoptionOutcome::Committed
        };
        original.style = editor.style;
        *editor = *self.original.take().unwrap();
        Ok(outcome)
    }
}

#[cfg(test)]
#[path = "editor_adoption_tests.rs"]
mod tests;

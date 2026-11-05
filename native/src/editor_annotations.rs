//! 仅标注替换：准备时冻结来源、提交核文档身份；底图、裁剪与逻辑画框始终由原Editor拥有。
use super::*;
use crate::edit_history::{Annotations, Entry};

/// 有界的一次性候选，不持来源栅格/历史，不允许按相同revision套用到另一份文档。
pub struct AnnotationReplacement {
    expected: DocumentStamp,
    annotations: Annotations,
    points: usize,
}
impl Editor {
    pub fn prepare_annotation_replacement(
        &self,
        source: &Editor,
    ) -> Result<AnnotationReplacement, EditError> {
        if self.draft.is_some() || source.draft.is_some() {
            return Err(EditError::InvalidPoint);
        }
        let points = source.elements.iter().map(|s| s.points.len()).sum();
        self.check_annotation_replacement(source.elements.len(), points)?;
        Ok(AnnotationReplacement {
            expected: self.document_stamp(),
            annotations: Annotations {
                elements: source.elements.clone(),
                serial: source.serial_count,
            },
            points,
        })
    }
    fn check_annotation_replacement(&self, count: usize, points: usize) -> Result<(), EditError> {
        if count > MAX_ELEMENTS
            || self
                .retained_points
                .checked_add(points)
                .is_none_or(|n| n > MAX_TOTAL_POINTS)
        {
            return Err(EditError::BudgetExceeded);
        }
        Ok(())
    }
}
impl AnnotationReplacement {
    /// 宿主仍须先核自身工作区/窗口及取消围栏；这里仅负责Editor身份和已提交文档事务。
    pub fn apply(self, target: &mut Editor) -> Result<bool, EditError> {
        self.apply_rebased(target, None)
    }
    /// 接收协作首次冻结basis时，矢量和已准备底图必须原子发布；所有可恢复拒绝先于交换。
    pub(crate) fn apply_rebased(
        self,
        target: &mut Editor,
        base: Option<crate::pixels::Pixels>,
    ) -> Result<bool, EditError> {
        if !self.expected.matches(target) || target.draft.is_some() {
            return Ok(false);
        }
        target.check_annotation_replacement(self.annotations.elements.len(), self.points)?;
        if base.as_ref().is_some_and(|base| {
            !target.source_replaceable()
                || target
                    .layers()
                    .is_none_or(|layers| layers.base.len() != base.len())
                || self.annotations.elements.iter().any(|s| s.blur.is_some())
        }) {
            return Ok(false);
        }
        // 以下只转移所有权，所有可恢复校验都已完成；原历史和工具样式不接管来源状态。
        let previous = Annotations {
            elements: std::mem::replace(&mut target.elements, self.annotations.elements),
            serial: std::mem::replace(&mut target.serial_count, self.annotations.serial),
        };
        target.retained_points += self.points;
        target.retained_points -= target.history.record(Entry::annotations(previous));
        if let Some(base) = base {
            target.history.rebase_source(&base);
            target.raster.as_mut().unwrap().base = base;
        }
        target.revision = target.revision.wrapping_add(1);
        Ok(true)
    }
}

#[cfg(test)]
#[path = "editor_annotations_tests.rs"]
mod tests;

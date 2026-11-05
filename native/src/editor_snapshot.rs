//! 已提交内容快照：共享像素，隔离矢量、历史、草稿和文档身份。
use super::*;

impl Editor {
    /// Bind a display-only snapshot to its pixels without replacing existing raster edits.
    pub fn duplicate_snapshot_with_fallback_base(
        &self,
        base: crate::erase_layers::Layers,
    ) -> Result<Self, EditError> {
        if self.raster.is_none() && !base.valid() {
            return Err(EditError::InvalidPoint);
        }
        let mut snapshot = self.duplicate_snapshot()?;
        if snapshot.raster.is_none() {
            snapshot.raster = Some(base);
        }
        Ok(snapshot)
    }
    /// A pasted document owns fresh annotation identities and no source undo history.
    pub fn duplicate_snapshot(&self) -> Result<Self, EditError> {
        let mut snapshot = self.export_snapshot();
        for stroke in &mut snapshot.elements {
            stroke.annotation_id = annotation_id::AnnotationId::new()?;
        }
        Ok(snapshot)
    }
    /// 导出候选独立替换效果资源；失败丢弃局部快照，不写入活动历史或修订号。
    pub fn export_snapshot_with_blurs<E>(
        &self,
        mut prepare: impl FnMut(
            &crate::blur_image::BlurImage,
        ) -> Result<crate::blur_image::BlurImage, E>,
    ) -> Result<Self, E> {
        self.export_snapshot_with_blur_strokes(|stroke| prepare(stroke.blur.as_ref().unwrap()))
    }
    /// 按稳定标注身份准备局部效果；闭包只读原 stroke，替换仅发生在独立快照内。
    pub fn export_snapshot_with_blur_strokes<E>(
        &self,
        mut prepare: impl FnMut(&Stroke) -> Result<crate::blur_image::BlurImage, E>,
    ) -> Result<Self, E> {
        let mut snapshot = self.export_snapshot();
        for stroke in &mut snapshot.elements {
            if stroke.blur.is_some() {
                stroke.blur = Some(prepare(stroke)?);
            }
        }
        Ok(snapshot)
    }
    /// 临时编辑 Art 覆盖层时绑定当前显示底图；不改变原文档或把旧栅格内容带入。
    pub fn export_overlay_with_base(
        &self,
        base: crate::erase_layers::Layers,
        measure: impl FnMut(&crate::text_annotation::TextAnnotation) -> Result<Point, EditError>,
    ) -> Result<Self, EditError> {
        if !base.valid() {
            return Err(EditError::InvalidPoint);
        }
        let mut snapshot = self.export_overlay_for_size(base.size, measure)?;
        snapshot.raster = Some(base);
        Ok(snapshot)
    }
    /// Art 显示只使用矢量/裁剪覆盖层；底图由生成结果提供，不带入旧栅格历史。
    pub fn export_overlay_snapshot(&self) -> Self {
        let mut snapshot = self.export_snapshot();
        snapshot.raster = None;
        snapshot
    }
    /// 协作basis只移除可共享矢量；已物化栅格、裁剪和逻辑几何必须原样保留。
    pub fn export_without_annotations(&self) -> Self {
        let mut snapshot = self.export_snapshot();
        snapshot.elements.clear();
        snapshot.retained_points = 0;
        snapshot
    }
    /// 导出只复制已提交的标注，不包含草稿、重做分支或图像像素。
    pub fn export_snapshot(&self) -> Self {
        let elements = self.elements().to_vec();
        let retained_points = elements.iter().map(|stroke| stroke.points.len()).sum();
        Self {
            style: self.style,
            identity: std::sync::Arc::default(),
            revision: self.revision,
            serial_count: self.serial_count,
            history: History::default(),
            elements,
            retained_points,
            draft: None,
            draft_motion: None,
            raster: self.raster.clone(),
            crop: self.crop,
            logical_size: self.logical_size,
        }
    }
}

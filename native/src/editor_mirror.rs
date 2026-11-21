//! 已物化像素的镜像朝向只投影按钮状态，绝不再次驱动渲染或 canonical flip。
use super::*;
use serde::{Deserialize, Serialize};

const FIELD: &str = "otterNativeMaterializedMirror";

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
pub(crate) struct MirrorState {
    pub x: bool,
    pub y: bool,
}
impl MirrorState {
    pub(super) fn toggled(mut self, axis: crate::raster_flip::Axis) -> Self {
        match axis {
            crate::raster_flip::Axis::Horizontal => self.x = !self.x,
            crate::raster_flip::Axis::Vertical => self.y = !self.y,
        }
        self
    }
    /// 旧主干只有 canonical 标志；像素仍由原 session_image_flip 物化一次。
    pub(crate) fn read_raw(
        raw: Option<&serde_json::value::RawValue>,
    ) -> Result<Self, &'static str> {
        #[derive(Default, Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Stored {
            flipped_x: Option<bool>,
            flipped_y: Option<bool>,
            otter_native_materialized_mirror: Option<MirrorState>,
        }
        let stored: Stored = raw
            .map(|raw| serde_json::from_str(raw.get()))
            .transpose()
            .map_err(|_| "Invalid materialized mirror state")?
            .unwrap_or_default();
        Ok(stored.otter_native_materialized_mirror.unwrap_or(Self {
            x: stored.flipped_x.unwrap_or(false),
            y: stored.flipped_y.unwrap_or(false),
        }))
    }
    /// 保留其它 imageEditState 扩展；包括 false，避免沿用原文的旧朝向。
    pub(crate) fn write(self, state: &mut serde_json::Value) -> Result<(), &'static str> {
        state
            .as_object_mut()
            .ok_or("Invalid image edit state")?
            .insert(FIELD.into(), serde_json::json!({"x": self.x, "y": self.y}));
        Ok(())
    }
}
impl Editor {
    pub(crate) fn mirror_state(&self) -> MirrorState {
        self.mirror
    }
    /// 只允许会话候选初始化；活动文档的朝向必须和像素事务一起提交。
    pub(crate) fn with_restored_mirror(mut self, mirror: MirrorState) -> Result<Self, EditError> {
        if self.revision != 0 || !self.history.is_empty() || self.draft.is_some() {
            return Err(EditError::InvalidPoint);
        }
        self.mirror = mirror;
        Ok(self)
    }
}

#[cfg(test)]
#[path = "editor_mirror_tests.rs"]
mod tests;

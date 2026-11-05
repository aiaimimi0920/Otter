//! 冻结文档的标注编码；普通视口与缩略展开参考框分别映射，字体测量与恢复端共用。
use super::Document;
use crate::{
    session_placement::Space,
    session_vector_write::{self, Error, Mapping},
};
use serde_json::{Map, Value};
use std::sync::atomic::{AtomicBool, Ordering};

impl Document {
    pub(super) fn annotation_mapping(
        &self,
        fields: &Map<String, Value>,
        space: Space,
    ) -> Result<Mapping, crate::session_write::Error> {
        match (self.minified.is_some(), self.transition.is_some()) {
            (false, false) => Mapping::from_viewport(self.viewport, self.dpi, space),
            (true, true) => {
                let reference = fields
                    .get("savedRect")
                    .and_then(Value::as_object)
                    .unwrap_or(fields);
                let dimension = |key| {
                    reference
                        .get(key)
                        .and_then(Value::as_f64)
                        .ok_or(crate::session_write::Error::Window)
                };
                Mapping::from_annotation_frame(
                    self.editor
                        .annotation_frame()
                        .or_else(|| self.editor.visible_frame()),
                    crate::crop_geometry::Size {
                        width: dimension("w")?,
                        height: dimension("h")?,
                    },
                )
            }
            _ => return Err(crate::session_write::Error::Window),
        }
        .map_err(|_| crate::session_write::Error::Window)
    }
    /// 只更新 annotationState；图像、裁剪与磁盘提交仍由完整内容编码 owner 负责。
    pub fn write_annotations(
        &self,
        mut fields: Map<String, Value>,
        space: Space,
        cancel: &AtomicBool,
    ) -> Result<Map<String, Value>, crate::session_write::Error> {
        let check = || {
            if cancel.load(Ordering::Acquire) {
                Err(crate::session_write::Error::Cancelled)
            } else {
                Ok(())
            }
        };
        check()?;
        let mapping = self.annotation_mapping(&fields, space)?;
        let original = fields
            .remove("annotationState")
            .filter(|value| !value.is_null())
            .map(|value| serde_json::value::to_raw_value(&value))
            .transpose()
            .map_err(|_| crate::session_write::Error::Window)?;
        check()?;
        let bytes = session_vector_write::encode_mapped_cancellable(
            &self.editor,
            original.as_deref(),
            mapping,
            |text| {
                if cancel.load(Ordering::Acquire) {
                    return Err(Error::Cancelled);
                }
                crate::desktop::annotation_text::measure_flip_metrics(text)
                    .map(|metrics| metrics.baseline_from_top)
                    .map_err(|_| Error::Geometry)
            },
            || cancel.load(Ordering::Acquire),
        );
        check()?;
        let bytes = bytes.map_err(|_| crate::session_write::Error::Window)?;
        let value =
            serde_json::from_slice(&bytes).map_err(|_| crate::session_write::Error::Window)?;
        check()?;
        fields.insert("annotationState".into(), value);
        Ok(fields)
    }
}

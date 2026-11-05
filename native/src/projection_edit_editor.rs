//! View与现有Editor的标注适配；basis/原字段仍为真值，不引入第二编辑器或栅格写入。
use super::{view::View, Objects};
use crate::{
    editor::{AnnotationReplacement, DocumentStamp, Editor},
    session_vectors::{self, TextMetrics},
    text_annotation::TextAnnotation,
};
use serde_json::{json, Value};
#[path = "projection_edit_editor_fields.rs"]
mod fields;

#[derive(Debug, PartialEq)]
pub enum Error {
    View(super::view::Error),
    Read(session_vectors::Error),
    Write(crate::session_vector_write::Error),
    Edit(crate::editor::EditError),
    Identity,
    Unsupported,
    Limit,
}
pub struct Prepared {
    source: View,
    expected: Objects,
    replacement: AnnotationReplacement,
    mapping: Option<crate::session_vector_write::Mapping>,
}
/// expected仅为f32 Editor投影；source保留未修改wire的f64/缺省/颜色词法与稳定层级。
pub struct Bridge {
    source: View,
    expected: Objects,
    document: DocumentStamp,
    mapping: Option<crate::session_vector_write::Mapping>,
}
pub fn prepare(
    view: &View,
    target: &Editor,
    measure: impl FnMut(&TextAnnotation) -> Result<TextMetrics, session_vectors::Error>,
) -> Result<Prepared, Error> {
    prepare_in(view, target, None, measure)
}
/// 恢复后的Editor可有canonical留白/尺度；复用原session映射，不把位置误当basis缩放。
pub fn prepare_mapped(
    view: &View,
    target: &Editor,
    mapping: crate::session_vector_write::Mapping,
    measure: impl FnMut(&TextAnnotation) -> Result<TextMetrics, session_vectors::Error>,
) -> Result<Prepared, Error> {
    prepare_in(view, target, Some(mapping), measure)
}
fn prepare_in(
    view: &View,
    target: &Editor,
    mapping: Option<crate::session_vector_write::Mapping>,
    mut measure: impl FnMut(&TextAnnotation) -> Result<TextMetrics, session_vectors::Error>,
) -> Result<Prepared, Error> {
    let local = view.local_objects().map_err(Error::View)?;
    let raw = serde_json::value::to_raw_value(&json!({"serialCounter":target.serial_count(),"elements":local.values().collect::<Vec<_>>()})).map_err(|_| Error::Limit)?;
    let mut restored = session_vectors::restore_with_text(Some(&raw), &mut measure)
        .map_err(Error::Read)?
        .editor;
    if let Some(mapping) = mapping {
        let local = encoded(
            &restored,
            &mut measure,
            Some(mapping.inverse().map_err(Error::Write)?),
        )?;
        let raw = serde_json::value::to_raw_value(&json!({"serialCounter":target.serial_count(),"elements":local.values().collect::<Vec<_>>()}))
            .map_err(|_| Error::Limit)?;
        restored = session_vectors::restore_with_text(Some(&raw), &mut measure)
            .map_err(Error::Read)?
            .editor;
    }
    let expected = encoded(&restored, &mut measure, mapping)?;
    Ok(Prepared {
        source: view.clone(),
        expected,
        replacement: target
            .prepare_annotation_replacement(&restored)
            .map_err(Error::Edit)?,
        mapping,
    })
}
impl Prepared {
    /// 宿主还须核工作区/窗口/取消；过期或换文档不产生bridge，不发布部分标注。
    pub fn apply(self, target: &mut Editor) -> Result<Option<Bridge>, Error> {
        self.apply_rebased(target, None)
    }
    pub(crate) fn apply_rebased(
        self,
        target: &mut Editor,
        base: Option<crate::pixels::Pixels>,
    ) -> Result<Option<Bridge>, Error> {
        if !self
            .replacement
            .apply_rebased(target, base)
            .map_err(Error::Edit)?
        {
            return Ok(None);
        }
        Ok(Some(Bridge {
            source: self.source,
            expected: self.expected,
            document: target.document_stamp(),
            mapping: self.mapping,
        }))
    }
}
impl Bridge {
    /// staged初始view早于网络；expected必须来自该view的Editor投影，不能把现在的Editor当旧基线。
    pub(crate) fn from_staged(
        source: View,
        editor: &Editor,
        mapping: crate::session_vector_write::Mapping,
        measure: impl FnMut(&TextAnnotation) -> Result<TextMetrics, session_vectors::Error>,
    ) -> Result<Self, Error> {
        let prepared = prepare_mapped(&source, editor, mapping, measure)?;
        Ok(Self {
            source: prepared.source,
            expected: prepared.expected,
            document: editor.document_stamp(),
            mapping: prepared.mapping,
        })
    }
    /// 仅宿主核过精确canonical内容/身份后绑定；不重写Editor或增加恢复undo。
    pub fn from_checkpoint(
        source: View,
        editor: &Editor,
        mapping: crate::session_vector_write::Mapping,
        mut measure: impl FnMut(&TextAnnotation) -> Result<TextMetrics, session_vectors::Error>,
    ) -> Result<Self, Error> {
        if editor.draft().is_some() {
            return Err(Error::Identity);
        }
        let mut expected = encoded(editor, &mut measure, Some(mapping))?;
        // source未观察过的Editor对象是本地新增，不可因expected=current而被当作缺失原值。
        expected.retain(|id, _| source.basis_objects().contains_key(id));
        Ok(Self {
            source,
            expected,
            document: editor.document_stamp(),
            mapping: Some(mapping),
        })
    }
    pub fn source(&self) -> &View {
        &self.source
    }
    /// 每次异步结果后由宿主重读当前Editor；不能拿旧快照盖掉保存期间的新修改或undo。
    pub fn current(
        &self,
        editor: &Editor,
        mut measure: impl FnMut(&TextAnnotation) -> Result<TextMetrics, session_vectors::Error>,
    ) -> Result<View, Error> {
        if !self.document.same_document(editor) || editor.draft().is_some() {
            return Err(Error::Identity);
        }
        let current = encoded(editor, &mut measure, self.mapping)?;
        let original = self.source.local_objects().map_err(Error::View)?;
        // 与canonical共用层级计划；按Editor实际顺序分配新增值，溢出时整体重编号。
        let order = crate::session_vector_write::OrderPlan::new(editor, original.values());
        let mut result = Objects::new();
        for (index, stroke) in editor.elements().iter().enumerate() {
            let id = stroke.annotation_id();
            let current = current.get(id).ok_or(Error::Identity)?;
            let mut value = fields::merge(original.get(id), self.expected.get(id), Some(current))
                .ok_or(Error::Unsupported)?;
            value["zIndex"] = order.value(id, index);
            fields::wire(&mut value)?;
            result.insert(id.to_owned(), value);
        }
        self.source.edited(result).map_err(Error::View)
    }
}
fn encoded(
    editor: &Editor,
    measure: &mut impl FnMut(&TextAnnotation) -> Result<TextMetrics, session_vectors::Error>,
    mapping: Option<crate::session_vector_write::Mapping>,
) -> Result<Objects, Error> {
    if editor.elements().len() > 256
        || editor.elements().iter().any(|s| {
            s.points.len() > 2048
                || s.text
                    .as_ref()
                    .is_some_and(|t| t.content().encode_utf16().count() > 4096)
        })
    {
        return Err(Error::Limit);
    }
    let mut baseline = |text: &TextAnnotation| {
        measure(text)
            .map(|v| v.baseline)
            .map_err(|_| crate::session_vector_write::Error::Geometry)
    };
    let bytes = match mapping {
        Some(mapping) => {
            crate::session_vector_write::encode_mapped(editor, None, mapping, &mut baseline)
        }
        None => crate::session_vector_write::encode(editor, None, &mut baseline),
    }
    .map_err(Error::Write)?;
    if bytes.len() > super::MAX_BYTES * 4 {
        return Err(Error::Limit);
    }
    let value: Value = serde_json::from_slice(&bytes).map_err(|_| Error::Unsupported)?;
    value["elements"]
        .as_array()
        .ok_or(Error::Unsupported)?
        .iter()
        .map(|v| {
            Ok((
                v["id"].as_str().ok_or(Error::Unsupported)?.to_owned(),
                v.clone(),
            ))
        })
        .collect()
}
#[cfg(test)]
#[path = "projection_edit_editor_tests.rs"]
mod tests;

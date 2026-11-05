//! 工作区内的协作权威/可见视图；不是凭据、自动重放许可或网络日志确认。
use super::{
    journal::Context,
    view::{Size, View},
    Code, Document, Objects, Route,
};
use crate::session_projection_scope::Scope;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
#[path = "projection_edit_workspace_record.rs"]
mod record;
#[path = "projection_edit_workspace_resolution.rs"]
mod resolution;
pub const FIELD: &str = "otterNativeProjectionEdit";
pub const MAX_RECORD: usize = super::MAX_BYTES * 3;
pub const MAX_ENTRIES: usize = 64;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    Source,
    Receiver,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Suspension {
    BasisChanged,
    UnsupportedLocal,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Snapshot {
    schema: String,
    context: Context,
    projection_id: String,
    route: Route,
    role: Role,
    document: Document,
    visible: Objects,
    conflict: Option<Code>,
    suspended: Option<Suspension>,
    layout: Layout,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Layout {
    width: f64,
    height: f64,
    scale: f32,
    offset_x: f32,
    offset_y: f32,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Format,
    Identity,
    Limit,
    ContentChanged,
}
impl Snapshot {
    pub fn new(
        context: Context,
        projection_id: &str,
        route: Route,
        role: Role,
        document: Document,
        visible: &View,
        conflict: Option<Code>,
    ) -> Result<Self, Error> {
        if document.basis() != visible.basis() {
            return Err(Error::Identity);
        }
        let value = Self {
            schema: "neuro.otter.projection-edit-workspace.v1".into(),
            context,
            projection_id: projection_id.into(),
            route,
            role,
            document,
            visible: visible.basis_objects().clone(),
            conflict,
            suspended: None,
            layout: Layout {
                width: visible.size().width(),
                height: visible.size().height(),
                scale: 1.0,
                offset_x: 0.0,
                offset_y: 0.0,
            },
        };
        value.validate()?;
        Ok(value)
    }
    fn validate(&self) -> Result<(), Error> {
        if self.schema != "neuro.otter.projection-edit-workspace.v1"
            || !super::projection(&self.projection_id)
            || Context::new(
                self.context.scope(),
                self.context.unit(),
                self.context.origin(),
                self.context.device(),
            )
            .is_err()
        {
            return Err(Error::Identity);
        }
        self.document.validate().map_err(|_| Error::Format)?;
        self.editor_view()?;
        self.to_view()?;
        self.view(Size::new(1.0, 1.0).map_err(|_| Error::Format)?)?;
        if serde_json::to_vec(self).map_or(true, |v| v.len() > MAX_RECORD - 128) {
            return Err(Error::Limit);
        }
        Ok(())
    }
    pub fn belongs_to(&self, unit: &str, scope: Scope) -> bool {
        self.context.unit() == unit && self.context.scope() == scope
    }
    pub fn document(&self) -> &Document {
        &self.document
    }
    pub fn context(&self) -> &Context {
        &self.context
    }
    pub fn projection_id(&self) -> &str {
        &self.projection_id
    }
    pub fn route(&self) -> Route {
        self.route
    }
    pub fn role(&self) -> Role {
        self.role
    }
    pub fn conflict(&self) -> Option<Code> {
        self.conflict
    }
    pub fn suspended(&self) -> Option<Suspension> {
        self.suspended
    }
    pub fn view(&self, size: Size) -> Result<View, Error> {
        View::from_basis(self.document.basis(), size, self.visible.clone())
            .map_err(|_| Error::Format)
    }
    pub fn editor_view(&self) -> Result<View, Error> {
        self.view(Size::new(self.layout.width, self.layout.height).map_err(|_| Error::Format)?)
    }
    pub fn to_view(&self) -> Result<crate::session_vector_write::Mapping, Error> {
        crate::session_vector_write::Mapping::from_components(
            self.layout.scale,
            crate::viewport::Point {
                x: self.layout.offset_x,
                y: self.layout.offset_y,
            },
        )
        .map_err(|_| Error::Format)
    }
    /// 首次接入沿像素fit的同一内容框；View尺寸不包括旧逻辑框中的letterbox。
    pub fn in_editor_frame(&self, frame: crate::crop_geometry::Rect) -> Result<Self, Error> {
        if self.layout.width != frame.width || self.layout.height != frame.height {
            return Err(Error::Identity);
        }
        let mut value = self.clone();
        value.layout.scale = 1.0;
        value.layout.offset_x = -frame.x as f32;
        value.layout.offset_y = -frame.y as f32;
        value.validate()?;
        Ok(value)
    }
    /// 源端可含crop平移和beautify留白；View按实际PNG逐轴密度映射，不猜全Editor框。
    pub(crate) fn in_source_layout(&self, layout: super::source::Layout) -> Result<Self, Error> {
        if self.editor_view()?.size() != layout.size().map_err(|_| Error::Format)? {
            return Err(Error::Identity);
        }
        let (scale, offset) = layout.mapping().map_err(|_| Error::Format)?.components();
        let mut value = self.clone();
        value.layout.scale = scale;
        value.layout.offset_x = offset.x;
        value.layout.offset_y = offset.y;
        value.validate()?;
        Ok(value)
    }
    /// 新canonical坐标先逆本次保存映射，再到原View参考空间；不将20DIP留白误当图像缩放。
    pub fn for_save(&self, mapping: crate::session_vector_write::Mapping) -> Result<Self, Error> {
        let (scale, offset) = mapping
            .inverse()
            .map_err(|_| Error::Format)?
            .then(self.to_view()?)
            .map_err(|_| Error::Format)?
            .components();
        let mut value = self.clone();
        value.layout.scale = scale;
        value.layout.offset_x = offset.x;
        value.layout.offset_y = offset.y;
        value.validate()?;
        Ok(value)
    }
    pub fn with_view(&self, view: &View) -> Result<Self, Error> {
        if self.document.basis() != view.basis()
            || view.size()
                != Size::new(self.layout.width, self.layout.height).map_err(|_| Error::Format)?
        {
            return Err(Error::Identity);
        }
        let value = Self {
            visible: view.basis_objects().clone(),
            ..self.clone()
        };
        value.validate()?;
        Ok(value)
    }
    pub fn suspend(&self, reason: Suspension) -> Self {
        Self {
            suspended: Some(reason),
            ..self.clone()
        }
    }
    /// read不确认任何网络pending；只推进同一basis/session的权威，当前本地值始终重读。
    pub fn received(&self, document: Document, current: &View) -> Result<Self, Error> {
        if self.suspended.is_some() {
            return Err(Error::ContentChanged);
        }
        let observed =
            View::from_document(&self.document, current.size()).map_err(|_| Error::Format)?;
        let mut conflict = self.conflict;
        if self.role == Role::Receiver
            && document.mode() == super::Mode::OneWay
            && !observed
                .changes_to(current)
                .map_err(|_| Error::Format)?
                .is_empty()
        {
            conflict = conflict.or(Some(Code::ReadOnly));
        }
        let merged =
            super::view::reconcile(&self.document, &observed, &document, current, conflict)
                .map_err(|_| Error::Identity)?;
        let value = Self {
            document,
            visible: merged.visible().basis_objects().clone(),
            conflict: merged.conflict().map(|v| v.code()),
            ..self.clone()
        };
        value.validate()?;
        Ok(value)
    }
    /// Apply只确认本请求实际发送的32项；其余及网络期间编辑仍由current覆盖，不误清本地变化。
    pub(crate) fn applied(
        &self,
        document: Document,
        previous: &Document,
        acknowledged: &View,
        current: &View,
    ) -> Result<Self, Error> {
        if self.suspended.is_some() || document.follows(&self.document).is_err() {
            return Err(Error::ContentChanged);
        }
        let merged =
            super::view::reconcile(previous, acknowledged, &document, current, self.conflict)
                .map_err(|_| Error::Identity)?;
        let value = Self {
            document,
            visible: merged.visible().basis_objects().clone(),
            conflict: merged.conflict().map(|v| v.code()),
            ..self.clone()
        };
        value.validate()?;
        Ok(value)
    }
    /// 编码器在图片/标注全部写好后调用；失败不交付部分工作区候选。
    pub fn install(&self, fields: &mut Map<String, Value>) -> Result<(), Error> {
        self.validate()?;
        record::install(self, fields)
    }
    /// 缺字段是旧普通工作区；显式null/坏记录/身份或内容不匹配均不静默修复。
    pub fn restore(
        window: &crate::session_window::Window<'_>,
        scope: Option<Scope>,
    ) -> Result<Option<Self>, Error> {
        let Some(raw) = window.otter_native_projection_edit else {
            return Ok(None);
        };
        let value = record::read(raw, window)?;
        value.validate()?;
        if !scope.is_some_and(|scope| value.belongs_to(&window.id, scope)) {
            return Err(Error::Identity);
        }
        Ok(Some(value))
    }
}

#[cfg(test)]
#[path = "projection_edit_workspace_tests.rs"]
mod tests;

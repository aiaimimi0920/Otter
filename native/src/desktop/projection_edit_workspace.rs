//! 平台协作会话只拥有权威记录与Editor桥；无网络/线程/凭据，失效后保存而不重放旧视图。
use crate::{
    editor::{DocumentStamp, Editor},
    pixels::Pixels,
    projection_edit::{
        editor::Bridge,
        workspace::{Snapshot, Suspension},
    },
    session_vectors::{Error as VectorError, TextMetrics},
    text_annotation::TextAnnotation,
};
use std::sync::{Arc, Weak};
use windows::{
    core::{Error, Result},
    Win32::Foundation::E_INVALIDARG,
};
#[path = "projection_edit_receiver.rs"]
pub(super) mod receiver;
#[path = "projection_edit_source.rs"]
pub(in crate::desktop) mod source;

#[derive(Clone)]
pub(super) struct Stamp(Arc<()>);
impl PartialEq for Stamp {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}
pub(super) struct Owner {
    snapshot: Snapshot,
    bridge: Option<Bridge>,
    base: Base,
    stamp: Stamp,
}
pub(in crate::desktop) struct Base {
    document: DocumentStamp,
    pixels: Option<Weak<crate::pixels::Storage>>,
    annotations: Option<Weak<crate::pixels::Storage>>,
    size: Option<(u32, u32)>,
    annotation_size: Option<(u32, u32)>,
    frame: Option<crate::crop_geometry::Rect>,
    crop: Option<crate::editor::CropView>,
}
impl Base {
    pub(in crate::desktop) fn capture(editor: &Editor) -> Self {
        let layers = editor.layers();
        Self {
            document: editor.document_stamp(),
            pixels: layers.map(|v| v.base.downgrade()),
            annotations: layers.and_then(|v| v.annotations.as_ref().map(Pixels::downgrade)),
            annotation_size: layers.map(|v| v.annotation_dimensions()),
            size: layers.map(|v| v.size),
            frame: editor.visible_frame(),
            crop: editor.crop_view(),
        }
    }
    pub(in crate::desktop) fn matches(&self, editor: &Editor) -> bool {
        let layers = editor.layers();
        let same = |a: Option<&Weak<crate::pixels::Storage>>, b: Option<&Pixels>| match (a, b) {
            (None, None) => true,
            (Some(a), Some(b)) => a.ptr_eq(&b.downgrade()),
            _ => false,
        };
        self.document.same_document(editor)
            && self.size == layers.map(|v| v.size)
            && self.annotation_size == layers.map(|v| v.annotation_dimensions())
            && self.frame == editor.visible_frame()
            && self.crop == editor.crop_view()
            && same(self.pixels.as_ref(), layers.map(|v| &v.base))
            && same(
                self.annotations.as_ref(),
                layers.and_then(|v| v.annotations.as_ref()),
            )
    }
}
impl Owner {
    pub(in crate::desktop) fn apply_response(
        &self,
        ticket: &crate::projection_edit::journal::Ticket,
        document: crate::projection_edit::Document,
        editor: &mut Editor,
    ) -> Result<Self> {
        let snapshot = self.capture(editor);
        if snapshot.context() != ticket.context()
            || snapshot.projection_id() != ticket.request().projection_id()
            || snapshot.route() != ticket.route()
        {
            return Err(invalid());
        }
        let current = snapshot.editor_view().map_err(|_| invalid())?;
        let acknowledged = crate::projection_edit::view::View::from_basis(
            snapshot.document().basis(),
            current.size(),
            ticket.view().clone(),
        )
        .map_err(|_| invalid())?;
        let next = snapshot
            .applied(
                document,
                ticket.previous().ok_or_else(invalid)?,
                &acknowledged,
                &current,
            )
            .map_err(|_| invalid())?;
        receiver::candidate(next, &current, editor)?.apply(editor, None)
    }
    pub(in crate::desktop) fn apply_ready(
        &self,
        ready: &crate::projection_delivery_runtime::edit::Ready,
        editor: &mut Editor,
    ) -> Result<Self> {
        if ready.resolution.is_none() && ready.rejection.is_none() {
            return self.apply_response(
                ready.ticket.as_ref().ok_or_else(invalid)?,
                ready.document.clone(),
                editor,
            );
        }
        let snapshot = self.capture(editor);
        let current = snapshot.editor_view().map_err(|_| invalid())?;
        let next = if let Some(resolution) = &ready.resolution {
            snapshot.resolved(
                ready.document.clone(),
                &resolution.observed,
                &current,
                resolution.keep_local,
            )
        } else {
            let ticket = ready.ticket.as_ref().ok_or_else(invalid)?;
            if snapshot.context() != ticket.context()
                || snapshot.route() != ticket.route()
                || snapshot.projection_id() != ticket.request().projection_id()
            {
                return Err(invalid());
            }
            snapshot
                .received(ready.document.clone(), &current)
                .and_then(|value| value.rejected(ready.rejection.unwrap()))
        }
        .map_err(|_| invalid())?;
        receiver::candidate(next, &current, editor)?.apply(editor, None)
    }
    /// 只操作未发布候选。完整恢复事务失败时，原Application/原图不变。
    pub(super) fn restore(snapshot: Snapshot, editor: Editor) -> Result<(Editor, Self)> {
        let (editor, bridge) = if snapshot.suspended().is_none() {
            if editor.crop_view().is_some()
                && snapshot.role() == crate::projection_edit::workspace::Role::Receiver
            {
                return Err(invalid());
            }
            let view = snapshot.editor_view().map_err(|_| invalid())?;
            let mapping = snapshot.to_view().map_err(|_| invalid())?;
            let bridge =
                Bridge::from_checkpoint(view, &editor, mapping, metrics).map_err(|_| invalid())?;
            (editor, Some(bridge))
        } else {
            (editor, None)
        };
        let base = Base::capture(&editor);
        Ok((
            editor,
            Self {
                snapshot,
                bridge,
                base,
                stamp: Stamp(Arc::default()),
            },
        ))
    }
    pub(super) fn stamp(&self) -> Stamp {
        self.stamp.clone()
    }
    pub(super) fn capture(&self, editor: &Editor) -> Snapshot {
        if self.snapshot.suspended().is_some() {
            return self.snapshot.clone();
        }
        // 不支持的本地底图/几何改变不能阻止正常保存，更不能在重开时用旧basis盖回新图。
        if !self.base.matches(editor) {
            return self.snapshot.suspend(Suspension::BasisChanged);
        }
        match self.bridge.as_ref().unwrap().current(editor, metrics) {
            Ok(view) => self
                .snapshot
                .with_view(&view)
                .unwrap_or_else(|_| self.snapshot.suspend(Suspension::UnsupportedLocal)),
            Err(_) => self.snapshot.suspend(Suspension::UnsupportedLocal),
        }
    }
}
pub(super) fn stamp(owner: Option<&Owner>) -> Option<Stamp> {
    owner.map(Owner::stamp)
}
fn invalid() -> Error {
    Error::new(E_INVALIDARG, "Invalid projection edit workspace state")
}
fn metrics(text: &TextAnnotation) -> std::result::Result<TextMetrics, VectorError> {
    unsafe {
        Ok(TextMetrics {
            size: super::annotation_text::measure(text).map_err(|_| VectorError::Geometry)?,
            baseline: super::annotation_text::measure_flip_metrics(text)
                .map_err(|_| VectorError::Geometry)?
                .baseline_from_top,
        })
    }
}

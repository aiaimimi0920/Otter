//! Process-local single-slot content; only a completed clipboard publication can bind it.
use super::{image::Image, open_image::PreparedDocument};
use crate::editor::Editor;
use std::{cell::RefCell, rc::Rc, sync::Arc};
use windows::core::{Error, Result};
use windows::Win32::Foundation::E_INVALIDARG;

pub(super) type Shared = Rc<RefCell<Cache>>;
pub(super) struct Content {
    pub image: Image,
    pub editor: Editor,
    pub opacity: f32,
    pub frame_style: crate::pin_frame_style::Style,
    pub capture_source: Option<super::canvas::CaptureSource>,
    pub origin: Option<super::clipboard_placement::Origin>,
}
pub(super) struct Prepared {
    pub editor: Editor,
    pub opacity: f32,
    pub frame_style: crate::pin_frame_style::Style,
}
#[derive(Default)]
pub(super) struct Cache {
    sequence: Option<u32>,
    entry: Option<Arc<Content>>,
    placement: Option<super::clipboard_placement::Session>,
}
impl Cache {
    pub fn published(&mut self, sequence: u32, content: Option<Arc<Content>>) {
        let origin = content.as_ref().and_then(|content| content.origin.clone());
        self.published_at(sequence, content, origin);
    }
    pub fn published_at(
        &mut self,
        sequence: u32,
        content: Option<Arc<Content>>,
        origin: Option<super::clipboard_placement::Origin>,
    ) {
        self.sequence = (sequence != 0).then_some(sequence);
        self.entry = content.filter(|_| sequence != 0);
        // 位置属于复制会话，而非可编辑内容；Art/小图扁平化不能丢失相对偏移。
        self.placement = origin
            .filter(|_| sequence != 0)
            .map(super::clipboard_placement::Session::new);
    }
    /// Called while the clipboard is locked; clone only the bounded shared owner.
    pub fn matching(&mut self, sequence: u32) -> Option<Arc<Content>> {
        if self.sequence == Some(sequence) && sequence != 0 {
            self.entry.clone()
        } else {
            self.entry = None;
            self.sequence = None;
            self.placement = None;
            None
        }
    }
    pub fn placement(
        &self,
        content: &Arc<Content>,
        cursor: (i32, i32),
    ) -> Option<super::clipboard_placement::Request> {
        self.entry
            .as_ref()
            .filter(|stored| Arc::ptr_eq(stored, content))?;
        self.position(cursor)
    }
    pub fn position(&self, cursor: (i32, i32)) -> Option<super::clipboard_placement::Request> {
        self.placement
            .as_ref()
            .map(|session| session.request(cursor))
    }
    pub fn refresh(
        &self,
        request: &super::clipboard_placement::Request,
    ) -> super::clipboard_placement::Request {
        self.placement
            .as_ref()
            .map_or_else(|| request.clone(), |session| session.refresh(request))
    }
    pub fn placed(
        &mut self,
        request: &super::clipboard_placement::Request,
        unit: crate::workspace_id::UnitId,
        frame: crate::pin_geometry::PinRect,
    ) {
        if let Some(session) = self.placement.as_mut() {
            session.commit(request, unit, frame);
        }
    }
}
impl Content {
    /// Run on the existing paste worker, never under the clipboard lock.
    pub fn prepare(&self, cancelled: impl Fn() -> bool) -> Result<PreparedDocument> {
        let editor = self
            .editor
            .duplicate_snapshot_with_fallback_base(crate::erase_layers::Layers {
                size: (self.image.width, self.image.height),
                base: self.image.pixels.clone(),
                annotations: None,
                annotation_size: None,
            })
            .map_err(|error| {
                Error::new(
                    E_INVALIDARG,
                    format!("Cannot duplicate clipboard edits: {error:?}"),
                )
            })?;
        if !self.opacity.is_finite() || !(0.0..=1.0).contains(&self.opacity) {
            return Err(Error::new(E_INVALIDARG, "Invalid clipboard opacity"));
        }
        let image = editor.layers().map_or_else(
            || self.image.clone(),
            |layers| Image {
                width: layers.size.0,
                height: layers.size.1,
                pixels: layers.base.clone(),
            },
        );
        let region = super::hit_region::HitRegion::prepare_cancellable(
            &image,
            editor
                .layers()
                .and_then(|layers| layers.annotations.as_ref()),
            editor
                .layers()
                .map_or((image.width, image.height), |layers| {
                    layers.annotation_dimensions()
                }),
            cancelled,
        )?;
        Ok(PreparedDocument {
            record_history: false,
            image,
            region,
            capture_origin: None,
            paste_origin: None,
            capture_source: self.capture_source.clone(),
            placement: None,
            editable: Some(Prepared {
                editor,
                opacity: self.opacity,
                frame_style: self.frame_style,
            }),
        })
    }
}

#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-capture")))]
#[path = "clipboard_editable_tests.rs"]
mod tests;

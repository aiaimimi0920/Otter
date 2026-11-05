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
    pub capture_source: Option<super::canvas::CaptureSource>,
    pub origin: Option<super::clipboard_placement::Origin>,
}
pub(super) struct Prepared {
    pub editor: Editor,
    pub opacity: f32,
}
#[derive(Default)]
pub(super) struct Cache {
    entry: Option<(u32, Arc<Content>)>,
    placement: Option<super::clipboard_placement::Session>,
}
impl Cache {
    pub fn published(&mut self, sequence: u32, content: Option<Arc<Content>>) {
        self.entry = content
            .filter(|_| sequence != 0)
            .map(|content| (sequence, content));
        self.placement = self
            .entry
            .as_ref()
            .and_then(|(_, content)| content.origin.clone())
            .map(super::clipboard_placement::Session::new);
    }
    /// Called while the clipboard is locked; clone only the bounded shared owner.
    pub fn matching(&mut self, sequence: u32) -> Option<Arc<Content>> {
        if self
            .entry
            .as_ref()
            .is_some_and(|(expected, _)| sequence != 0 && *expected == sequence)
        {
            self.entry.as_ref().map(|(_, content)| content.clone())
        } else {
            self.entry = None;
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
            .filter(|(_, stored)| Arc::ptr_eq(stored, content))?;
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
            capture_source: self.capture_source.clone(),
            placement: None,
            editable: Some(Prepared {
                editor,
                opacity: self.opacity,
            }),
        })
    }
}

#[cfg(test)]
#[path = "clipboard_editable_tests.rs"]
mod tests;

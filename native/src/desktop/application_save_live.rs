//! Live frame snapshots for session saving; UI transfers GPU owners only, and the existing save worker reads back and encodes.
use super::{Application, Document, Payload};
use crate::{
    desktop::composition::live::session::Session, live_frames::Frame, live_texture::Texture,
    pin_geometry::PinRect, workspace_id::UnitId,
};
use std::sync::{atomic::AtomicBool, Arc};
use windows::{
    core::{Error, Result},
    Win32::Foundation::E_ABORT,
};

#[derive(Clone)]
pub(super) struct Guard {
    unit: UnitId,
    size: [u32; 2],
    rect: PinRect,
    opacity: f32,
}
pub(super) struct Snapshot {
    guard: Guard,
    frame: Arc<Frame<Texture>>,
    editor: Option<crate::editor::Editor>,
    group: Option<UnitId>,
}
pub(super) struct Capture {
    pub(super) guards: Vec<Guard>,
    pub(super) frames: Vec<Snapshot>,
}

fn members(app: &Application) -> impl Iterator<Item = &Session> {
    // Recycling removes the graph node first; the worker may retain it until actual exit, but it no longer belongs to the next save.
    // Do not filter by is_alive; unexpectedly closed Live nodes still in the graph must continue failing closed.
    app.live_sessions.iter().filter(|session| {
        !session.pin_attached()
            && session.belongs_to(&app.workspace)
            && app.workspace.contains(session.unit_id())
    })
}

#[cfg(test)]
std::thread_local! {
    pub(super) static FRAME_READS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

fn observe(session: &Session, automatic: bool) -> Result<Option<(Arc<Frame<Texture>>, PinRect)>> {
    if automatic {
        if !session.persistence_idle() {
            return Ok(None);
        }
    } else if !session.is_alive() {
        return Err(Error::new(E_ABORT, "Live session is closing during save"));
    }
    #[cfg(test)]
    FRAME_READS.with(|reads| reads.set(reads.get() + 1));
    let Some(frame) = session.snapshot_frame()? else {
        return if automatic {
            Ok(None)
        } else {
            Err(Error::new(E_ABORT, "Live has no frame to save"))
        };
    };
    match session.freeze_rect([frame.resource.width, frame.resource.height]) {
        Ok(rect) => Ok(Some((frame, rect))),
        // Like first-frame waits and busy gestures, temporarily unfreezable geometry defers work without disabling autosave.
        Err(_) if automatic => Ok(None),
        Err(error) => Err(error),
    }
}
pub(super) fn automatic_ready(app: &Application) -> Result<bool> {
    for session in members(app) {
        if observe(session, true)?.is_none() {
            return Ok(false);
        }
    }
    Ok(true)
}
pub(super) fn capture(
    app: &Application,
    automatic: bool,
    selection: Payload<'_>,
) -> Result<Option<Capture>> {
    let mut frames = Vec::new();
    let mut guards = Vec::new();
    for session in members(app) {
        let Some((frame, rect)) = observe(session, automatic)? else {
            return Ok(None);
        };
        let guard = Guard {
            unit: session.unit_id().clone(),
            size: [frame.resource.width, frame.resource.height],
            rect,
            opacity: session
                .opacity()
                .ok_or_else(|| Error::new(E_ABORT, "Live opacity owner closed"))?,
        };
        // Every member contributes readiness/geometry, but only selected frames outlive capture.
        if selection.includes(session.unit_id()) {
            frames.push(Snapshot {
                guard: guard.clone(),
                frame,
                editor: session.pin_snapshot(),
                group: app
                    .workspace
                    .group_of(session.unit_id())
                    .map(|group| group.id.clone()),
            });
        }
        guards.push(guard);
    }
    if app.documents.len() + guards.len() > crate::workspace::MAX_UNITS {
        return Err(Error::new(E_ABORT, "Live save exceeds workspace capacity"));
    }
    Ok(Some(Capture { guards, frames }))
}
pub(super) fn matches(guards: &[Guard], app: &Application) -> bool {
    let sessions: Vec<_> = members(app).collect();
    sessions.len() == guards.len()
        && guards.iter().all(|guard| {
            sessions.iter().any(|session| {
                session.unit_id() == &guard.unit
                    && session.is_alive()
                    && session.opacity() == Some(guard.opacity)
                    && session
                        .freeze_rect(guard.size)
                        .is_ok_and(|rect| rect == guard.rect)
            })
        })
}
impl Snapshot {
    pub(super) fn materialize(
        self,
        cancel: &AtomicBool,
    ) -> std::result::Result<Document, crate::session_write::Error> {
        use crate::session_write::Error;
        let image = crate::live_texture::readback::read(self.frame, cancel).map_err(|error| {
            if matches!(error, crate::live_texture::readback::Error::Cancelled) {
                Error::Cancelled
            } else {
                Error::Window
            }
        })?;
        let image = crate::desktop::image::Image {
            width: image.width,
            height: image.height,
            pixels: image.pixels,
        };
        // Save the current frame and local annotations, not capture targets, authorization, or runtime handles.
        let (editor, image) = if let Some(editor) = self.editor {
            crate::desktop::live_pin_snapshot::refresh(editor, image, || {
                cancel.load(std::sync::atomic::Ordering::Acquire)
            })
            .map_err(|_| {
                if cancel.load(std::sync::atomic::Ordering::Acquire) {
                    Error::Cancelled
                } else {
                    Error::Window
                }
            })?
        } else {
            let editor = crate::editor::Editor::with_layers(crate::erase_layers::Layers {
                size: (image.width, image.height),
                base: image.pixels.clone(),
                annotation_size: None,
                annotations: None,
            })
            .map_err(|_| Error::Window)?;
            (editor, image)
        };
        Ok(Document {
            extensions: None,
            unit: self.guard.unit,
            frame: self.guard.rect,
            viewport: crate::viewport::Viewport {
                origin: Default::default(),
                scale: self.guard.rect.width as f32 / image.width as f32,
            },
            dpi: 96,
            editor,
            local_image: Some(image),
            restored_source: None,
            restored_source_rasterized_crop: false,
            output_images: Vec::new(),
            outputs: None,
            capture_original: None,
            capture_meta: None,
            minified: None,
            transition: None,
            normal_opacity: self.guard.opacity,
            mini_opacity: 0.9,
            frame_style: None,
            node_settings: None,
            pinned: true,
            art: false,
            local_art_id: None,
            local_art_ports: None,
            group: self.group,
            parameters: None,
            sticker: None,
            execution: None,
        })
    }
}

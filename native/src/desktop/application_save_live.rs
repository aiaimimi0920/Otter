//! 显式会话保存的 Live 帧快照；UI 只移交 GPU owner，既有保存 worker 读回并编码。
use super::{Application, Document};
use crate::{
    live_frames::Frame, live_texture::Texture, pin_geometry::PinRect, workspace_id::UnitId,
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
    group: Option<UnitId>,
}
pub(super) struct Snapshot {
    guard: Guard,
    frame: Arc<Frame<Texture>>,
}
fn owns_saved_unit(
    app: &Application,
    session: &crate::desktop::composition::live::session::Session,
) -> bool {
    // 回收先移除图节点，worker 可继续保留到实际退出；它不再是下一份保存的成员。
    // 不以 is_alive 过滤，仍在图内却意外关闭的 Live 必须继续失败关闭。
    session.belongs_to(&app.workspace) && app.workspace.contains(session.unit_id())
}
pub(super) fn automatic_ready(app: &Application) -> Result<bool> {
    for session in app
        .live_sessions
        .iter()
        .filter(|session| owns_saved_unit(app, session))
    {
        if !session.persistence_idle() {
            return Ok(false);
        }
        let Some(frame) = session.snapshot_frame()? else {
            return Ok(false); // 首帧/正常手势尚未提交，等原帧或输入消息，不停用自动保存。
        };
        if session
            .freeze_rect([frame.resource.width, frame.resource.height])
            .is_err()
        {
            return Ok(false);
        }
    }
    Ok(true)
}
pub(super) fn capture(app: &Application) -> Result<(Vec<Guard>, Vec<Snapshot>)> {
    let mut guards = Vec::new();
    let mut frames = Vec::new();
    for session in app
        .live_sessions
        .iter()
        .filter(|session| owns_saved_unit(app, session))
    {
        if !session.is_alive() {
            return Err(Error::new(E_ABORT, "Live session is closing during save"));
        }
        let frame = session
            .snapshot_frame()?
            .ok_or_else(|| Error::new(E_ABORT, "Live has no frame to save"))?;
        let size = [frame.resource.width, frame.resource.height];
        let rect = session.freeze_rect(size)?;
        let guard = Guard {
            unit: session.unit_id().clone(),
            size,
            rect,
            opacity: session
                .opacity()
                .ok_or_else(|| Error::new(E_ABORT, "Live opacity owner closed"))?,
            group: app
                .workspace
                .group_of(session.unit_id())
                .map(|group| group.id.clone()),
        };
        guards.push(guard.clone());
        frames.push(Snapshot { guard, frame });
    }
    if app.documents.len() + frames.len() > crate::workspace::MAX_UNITS {
        return Err(Error::new(E_ABORT, "Live save exceeds workspace capacity"));
    }
    Ok((guards, frames))
}
pub(super) fn matches(guards: &[Guard], app: &Application) -> bool {
    let sessions: Vec<_> = app
        .live_sessions
        .iter()
        .filter(|session| owns_saved_unit(app, session))
        .collect();
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
        // Live 不可编辑；保存为普通静态 sticker，不序列化采集目标、授权或运行句柄。
        let editor = crate::editor::Editor::with_layers(crate::erase_layers::Layers {
            size: (image.width, image.height),
            base: image.pixels.clone(),
            annotation_size: None,
            annotations: None,
        })
        .map_err(|_| Error::Window)?;
        Ok(Document {
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
            output_images: Vec::new(),
            outputs: None,
            capture_original: None,
            capture_meta: None,
            minified: None,
            transition: None,
            normal_opacity: self.guard.opacity,
            mini_opacity: 0.9,
            pinned: true,
            art: false,
            local_art_id: None,
            local_art_ports: None,
            group: self.guard.group,
            parameters: None,
            sticker: None,
            execution: None,
        })
    }
}

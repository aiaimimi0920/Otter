//! Session-local authority for optional image archives. The worker consumes the exact
//! published PNG or immutable screenshot, never a later view of a document.
use super::{display_snapshot::Stamp, export_job::ExportJob, image::Image, State};
use crate::{pixels::Pixels, workspace_id::UnitId};
use std::{
    cell::RefCell,
    collections::VecDeque,
    path::{Path, PathBuf},
    rc::{Rc, Weak},
    sync::{atomic::AtomicBool, Arc},
};
use windows::Win32::Foundation::{POINT, RECT};
pub(super) type Shared = Rc<RefCell<Owner>>;
#[path = "export_archive_file.rs"]
mod archive_file;
#[path = "export_archive_directory.rs"]
mod directory;
pub(super) use directory::grant;
pub(super) use directory::Target;
enum Source {
    Document(UnitId, Stamp),
    Frozen([i32; 4]),
}
struct Key {
    directory: PathBuf,
    id: directory::DirectoryId,
    source: Source,
}
impl Key {
    fn same(&self, other: &Self) -> bool {
        self.directory == other.directory
            && self.id == other.id
            && match (&self.source, &other.source) {
                (Source::Document(a, sa), Source::Document(b, sb)) => a == b && sa.same(sb),
                // A second frozen capture of the same bounds can contain a different
                // menu or animation. It must not be suppressed by a geometry-only key.
                _ => false,
            }
    }
}
enum Payload {
    Encoded(Pixels),
    Pin(
        Image,
        crate::editor::Editor,
        super::export::style::Snapshot,
        f32,
    ),
    Frozen(Image, RECT),
}
type FrozenCache =
    Arc<std::sync::Mutex<VecDeque<(PathBuf, directory::DirectoryId, [i32; 4], [u8; 32])>>>;
struct Request {
    key: Arc<Key>,
    target: Arc<Target>,
    payload: Payload,
    frozen: FrozenCache,
}
impl Request {
    fn write(self) -> Result<(), String> {
        let cancel = AtomicBool::new(false);
        let png = match self.payload {
            Payload::Encoded(png) => png,
            Payload::Pin(image, editor, style, opacity) => style
                .encoded_png(&image, &editor, opacity)
                .map_err(|e| e.to_string())?,
            Payload::Frozen(image, rect) => {
                let cropped = super::capture_crop::crop(&image, rect).map_err(|e| e.to_string())?;
                super::export::png_raw(&cropped, &cancel).map_err(|e| e.to_string())?
            }
        };
        let receipt = if let Source::Frozen(bounds) = &self.key.source {
            let digest = ring::digest::digest(&ring::digest::SHA256, &png);
            let key = (
                self.key.directory.clone(),
                self.key.id,
                *bounds,
                digest.as_ref().try_into().expect("SHA-256"),
            );
            if self
                .frozen
                .lock()
                .map_err(|_| "Invalid archive cache")?
                .contains(&key)
            {
                return Ok(());
            }
            Some(key)
        } else {
            None
        };
        let stem = if receipt.is_some() {
            "Otter_region"
        } else {
            "Otter_pin"
        };
        self.target
            .begin_file()
            .and_then(|stage| stage.publish(&png, stem, &cancel))
            .map_err(|e| e.to_string())?;
        if let Some(key) = receipt {
            let mut done = self.frozen.lock().map_err(|_| "Invalid archive cache")?;
            if done.len() == 256 {
                done.pop_front();
            }
            done.push_back(key);
        }
        Ok(())
    }
}
#[derive(Default)]
pub(super) struct Owner {
    pub copy: bool,
    pub pin: bool,
    pub target: Option<Arc<Target>>,
    profile: Option<PathBuf>,
    pending: VecDeque<Request>,
    active: Option<(Arc<Key>, ExportJob)>,
    done: VecDeque<Arc<Key>>,
    frozen: FrozenCache,
    pub error: Option<String>,
}
impl Owner {
    pub fn bind_profile(&mut self, profile: Option<&Path>) {
        self.profile = profile.map(Path::to_owned);
    }
    pub fn directory(&self) -> Result<PathBuf, String> {
        self.target
            .as_ref()
            .map(|t| t.path.clone())
            .or_else(|| self.profile.as_ref().map(|p| p.join("saved")))
            .ok_or_else(|| crate::ui_text::text("archive-select-directory").into())
    }
    pub fn confirm(&mut self, shown: &Path) -> Result<(), String> {
        if self.directory()? != shown {
            return Err(crate::ui_text::text("archive-select-directory").into());
        }
        if self.target.is_none() {
            self.target = Some(directory::grant_default(
                self.profile
                    .as_deref()
                    .expect("default directory requires profile"),
            )?);
        }
        Ok(())
    }
    pub fn busy(&self) -> bool {
        self.active.is_some() || !self.pending.is_empty()
    }
    fn submit(&mut self, source: Source, payload: Payload) {
        let Some(target) = self.target.clone() else {
            self.error = Some(crate::ui_text::text("archive-select-directory").into());
            return;
        };
        let key = Arc::new(Key {
            directory: target.path.clone(),
            id: target.id,
            source,
        });
        if self
            .done
            .iter()
            .chain(self.pending.iter().map(|r| &r.key))
            .chain(self.active.iter().map(|(k, _)| k))
            .any(|k| k.same(&key))
        {
            return;
        }
        if self.pending.len() >= 4 {
            self.error = Some(crate::ui_text::text("archive-queue-full").into());
            return;
        }
        let request = Request {
            key,
            target,
            payload,
            frozen: self.frozen.clone(),
        };
        self.pending.push_back(request);
        self.pump(false);
    }
    pub fn pump(&mut self, drain: bool) {
        loop {
            if let Some((_, job)) = &self.active {
                if !drain && !job.is_finished() {
                    return;
                }
            }
            if let Some((key, job)) = self.active.take() {
                match job.complete() {
                    Ok(()) => {
                        if self.done.len() == 256 {
                            self.done.pop_front();
                        }
                        self.done.push_back(key);
                    }
                    Err(e) => self.error = Some(e),
                }
            }
            let Some(request) = self.pending.pop_front() else {
                return;
            };
            let key = request.key.clone();
            match ExportJob::start(move || request.write()) {
                Ok(job) => {
                    self.active = Some((key, job));
                    // Let the pump observe completion on a later turn. Fast
                    // encodes must not make one UI turn drain the entire queue.
                    if !drain {
                        return;
                    }
                }
                Err(e) => self.error = Some(e),
            }
        }
    }
}
pub(super) fn copied(state: &State, stamp: Stamp, png: Pixels) {
    let mut archive = state.image_archive.borrow_mut();
    if archive.copy {
        archive.submit(
            Source::Document(state.unit_id.clone(), stamp),
            Payload::Encoded(png),
        );
    }
}
pub(super) fn pinned(state: &State) {
    let mut archive = state.image_archive.borrow_mut();
    if !archive.pin {
        return;
    }
    let canvas = state.canvas.borrow();
    if let Some(image) = &canvas.image {
        archive.submit(
            Source::Document(state.unit_id.clone(), Stamp::capture(&canvas, None)),
            Payload::Pin(
                image.clone(),
                canvas.editor.export_snapshot(),
                super::export::style::Snapshot::capture(&canvas),
                canvas.normal_opacity,
            ),
        );
    }
}
thread_local! {static SELECTOR:RefCell<Weak<RefCell<Owner>>>=Default::default();}
pub(super) struct Selector(Weak<RefCell<Owner>>);
impl Selector {
    pub fn enter(shared: &Shared) -> Self {
        Self(SELECTOR.replace(Rc::downgrade(shared)))
    }
}
impl Drop for Selector {
    fn drop(&mut self) {
        SELECTOR.replace(std::mem::take(&mut self.0));
    }
}
pub(super) fn frozen_enabled() -> bool {
    SELECTOR.with_borrow(|w| w.upgrade().is_some_and(|s| s.borrow().copy))
}
pub(super) fn frozen(image: &Image, rect: RECT, origin: POINT) {
    if let Some(owner) = SELECTOR.with_borrow(Weak::upgrade) {
        let mut owner = owner.borrow_mut();
        if owner.copy {
            let Ok(r) = super::capture_crop::screen_rectangle(origin, rect) else {
                return;
            };
            owner.submit(
                Source::Frozen([r.left, r.top, r.right, r.bottom]),
                Payload::Frozen(image.clone(), rect),
            );
        }
    }
}
#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-capture")))]
#[path = "export_image_autosave_tests.rs"]
mod tests;
